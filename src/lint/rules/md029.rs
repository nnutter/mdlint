use crate::lint::rule::Rule;
use crate::markdown::MarkdownParser;
use crate::types::{Fix, Violation};
use pulldown_cmark::{Event, Tag, TagEnd};
use serde_json::Value;

pub struct MD029;

impl Rule for MD029 {
    fn name(&self) -> &'static str {
        "MD029"
    }

    fn description(&self) -> &'static str {
        "Ordered list item prefix"
    }

    fn tags(&self) -> &[&str] {
        &["ol"]
    }

    fn check(&self, parser: &MarkdownParser, config: Option<&Value>) -> Vec<Violation> {
        let style = config
            .and_then(|c| c.get("style"))
            .and_then(|v| v.as_str())
            .unwrap_or("one");

        let mut violations = Vec::new();
        // Stack: None = unordered list, Some((expected_next, seen_non_one)) = ordered list.
        // Using AST events rather than raw line scanning ensures that code blocks, headings,
        // and other block-level elements correctly break list continuity.
        let mut list_stack: Vec<Option<(usize, bool, bool)>> = Vec::new();

        for (event, range) in parser.parse_with_offsets() {
            match event {
                Event::Start(Tag::List(start)) => {
                    if let Some(start) = start {
                        let start =
                            usize::try_from(start).expect("CommonMark list numbers fit usize");
                        list_stack.push(Some((start, false, true)));
                    } else {
                        list_stack.push(None);
                    }
                }
                Event::End(TagEnd::List(_)) => {
                    list_stack.pop();
                }
                Event::Start(Tag::Item) => {
                    if let Some(Some((expected, seen_non_one, first))) = list_stack.last_mut() {
                        let (line_num, column) = parser.offset_to_position(range.start);
                        if let Some(line) = parser.get_line(line_num)
                            && let Some(item) = line.get(column - 1..)
                            && let Some(num) = parse_item_number(item)
                        {
                            if num != 1 && !*first {
                                *seen_non_one = true;
                            }

                            let is_valid = match style {
                                "one" => num == if *first { *expected } else { 1 },
                                "ordered" => num == *expected,
                                _ => {
                                    // "one_or_ordered": if we've seen non-1, require sequential;
                                    // otherwise allow either all-ones or sequential.
                                    if *seen_non_one {
                                        num == *expected
                                    } else {
                                        num == 1 || num == *expected
                                    }
                                }
                            };

                            if !is_valid {
                                let should_be = if style == "one" && !*first {
                                    1
                                } else {
                                    *expected
                                };
                                let indent = column - 1;
                                let digit_len =
                                    item.chars().take_while(char::is_ascii_digit).count();
                                violations.push(Violation {
                                    line: line_num,
                                    column: Some(indent + 1),
                                    rule: self.name().to_owned(),
                                    message: format!(
                                        "Ordered list item prefix: expected {should_be}, found {num}"
                                    ),
                                    fix: Some(Fix {
                                        line_start: line_num,
                                        line_end: line_num,
                                        column_start: Some(indent + 1),
                                        column_end: Some(indent + digit_len),
                                        replacement: should_be.to_string(),
                                        description: format!(
                                            "Renumber ordered list item to {should_be}"
                                        ),
                                    }),
                                });
                            }

                            *expected += 1;
                            *first = false;
                        }
                    }
                }
                _ => {}
            }
        }

        violations
    }

    fn fixable(&self) -> bool {
        true
    }
}

/// Extract the leading integer from an ordered list item line (after stripping indentation).
/// Returns `Some(n)` for `"3. text"` or `"3) text"`, `None` otherwise.
fn parse_item_number(trimmed: &str) -> Option<usize> {
    let digits: String = trimmed.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return None;
    }
    let rest = &trimmed[digits.len()..];
    if rest.starts_with(['.', ')']) {
        digits.parse().ok()
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fix::Fixer;
    use crate::lint::rules::rendered;
    use indoc::indoc;

    fn apply_fixes(content: &str, violations: &[Violation]) -> String {
        let fixes: Vec<_> = violations.iter().filter_map(|v| v.fix.clone()).collect();
        Fixer::new()
            .apply_fixes_to_content(content, &fixes)
            .unwrap()
    }

    #[test]
    fn test_ordered_sequence() {
        let content = indoc! {"
            1. First
            2. Second
            3. Third"};
        let parser = MarkdownParser::new(content);
        let rule = MD029;
        let config = serde_json::json!({ "style": "ordered" });
        let violations = rule.check(&parser, Some(&config));

        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_all_ones() {
        let content = indoc! {"
            1. First
            1. Second
            1. Third"};
        let parser = MarkdownParser::new(content);
        let rule = MD029;
        let violations = rule.check(&parser, None);

        assert!(violations.is_empty());
    }

    #[test]
    fn test_wrong_sequence() {
        let content = indoc! {"
            1. First
            3. Third - wrong
            4. Fourth"};
        let parser = MarkdownParser::new(content);
        let rule = MD029;
        let violations = rule.check(&parser, None);

        assert_eq!(
            rendered(&violations),
            [
                "test.md:2:1: MD029 Ordered list item prefix: expected 1, found 3",
                "test.md:3:1: MD029 Ordered list item prefix: expected 1, found 4",
            ]
        );
    }

    #[test]
    fn test_enforced_ordered() {
        let content = "1. First\n1. Second - should be 2";
        let parser = MarkdownParser::new(content);
        let rule = MD029;
        let config = serde_json::json!({ "style": "ordered" });
        let violations = rule.check(&parser, Some(&config));

        assert_eq!(
            rendered(&violations),
            ["test.md:2:1: MD029 Ordered list item prefix: expected 2, found 1"]
        );
    }

    #[test]
    fn test_enforced_one() {
        let content = "1. First\n2. Second - should be 1";
        let parser = MarkdownParser::new(content);
        let rule = MD029;
        let config = serde_json::json!({ "style": "one" });
        let violations = rule.check(&parser, Some(&config));

        assert_eq!(
            rendered(&violations),
            ["test.md:2:1: MD029 Ordered list item prefix: expected 1, found 2"]
        );
    }

    #[test]
    fn test_list_with_backticks() {
        // Test numbered list where items contain backticks
        let content = indoc! {"
            1. Command-line options (`--config`)
            2. Local directory config (`mdlint.toml` in current dir)
            3. Parent directory configs (walking up to root)
            4. Default configuration"};
        let parser = MarkdownParser::new(content);
        let rule = MD029;
        let config = serde_json::json!({ "style": "ordered" });
        let violations = rule.check(&parser, Some(&config));

        assert_eq!(violations.len(), 0, "List with backticks should be valid");
    }

    #[test]
    fn test_fix_populated_for_wrong_number() {
        let content = "1. First\n2. Second\n3. Third";
        let violations = MD029.check(&MarkdownParser::new(content), None);
        assert_eq!(
            rendered(&violations),
            [
                "test.md:2:1: MD029 Ordered list item prefix: expected 1, found 2",
                "test.md:3:1: MD029 Ordered list item prefix: expected 1, found 3",
            ]
        );
        assert_eq!(
            apply_fixes(content, &violations),
            "1. First\n1. Second\n1. Third"
        );
    }

    #[test]
    fn test_fix_indented_list() {
        let content = indoc! {"
            1. First

               text

            2. Second"};
        let parser = MarkdownParser::new(content);
        let rule = MD029;
        let violations = rule.check(&parser, None);

        assert_eq!(
            rendered(&violations),
            ["test.md:5:1: MD029 Ordered list item prefix: expected 1, found 2"]
        );
        let fix = violations[0].fix.as_ref().expect("fix should be Some");
        assert_eq!(fix.replacement, "1");
        assert_eq!(fix.column_start, Some(1));
    }

    #[test]
    fn test_fix_renumbers_list() {
        let content = indoc! {"
            1. First
            2. Second
            3. Third
        "};
        let parser = MarkdownParser::new(content);
        let rule = MD029;
        let violations = rule.check(&parser, None);
        let fixed = apply_fixes(content, &violations);
        assert_eq!(
            fixed,
            indoc! {"
                1. First
                1. Second
                1. Third
            "}
        );
    }

    #[test]
    fn test_code_block_breaks_list() {
        // An unindented code block breaks CommonMark list continuity.
        // Each numbered item is its own single-item list starting at 1.
        let content = indoc! {"
            1. First item

            ```
            code
            ```

            1. Second item

            ```
            code
            ```

            1. Third item
        "};
        let parser = MarkdownParser::new(content);
        let rule = MD029;
        let violations = rule.check(&parser, None);

        // Each `1.` is the first item of a fresh list — no violations.
        assert_eq!(violations.len(), 0);
    }
}
