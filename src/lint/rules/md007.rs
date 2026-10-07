use crate::lint::rule::Rule;
use crate::markdown::MarkdownParser;
use crate::types::Violation;
use pulldown_cmark::{Event, Tag, TagEnd};
use serde_json::Value;

pub struct MD007;

struct ListIndentation {
    ordered: bool,
    marker: usize,
    content: usize,
}

impl Rule for MD007 {
    fn name(&self) -> &'static str {
        "MD007"
    }

    fn description(&self) -> &'static str {
        "Unordered list indentation"
    }

    fn tags(&self) -> &[&str] {
        &["bullet", "ul", "indentation"]
    }

    #[allow(clippy::cast_possible_truncation)] // serde_json gives u64; values are small config counts
    fn check(&self, parser: &MarkdownParser, config: Option<&Value>) -> Vec<Violation> {
        let indent_size = config
            .and_then(|c| c.get("indent"))
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(2) as usize;

        let mut violations = Vec::new();
        let mut lists: Vec<ListIndentation> = Vec::new();
        for (event, range) in parser.parse_with_offsets() {
            match event {
                Event::Start(Tag::List(start)) => lists.push(ListIndentation {
                    ordered: start.is_some(),
                    marker: 0,
                    content: 0,
                }),
                Event::End(TagEnd::List(_)) => {
                    lists.pop();
                }
                Event::Start(Tag::Item) => {
                    let (line_number, column) = parser.offset_to_position(range.start);
                    let Some(line) = parser.get_line(line_number) else {
                        continue;
                    };
                    let remaining = &line[column - 1..];
                    let item = remaining.trim_start();
                    let marker_column = column - 1 + remaining.len() - item.len();
                    let prefix = &line[..marker_column];
                    let quote_width = prefix.rfind('>').map_or(0, |end| {
                        end + 1 + usize::from(prefix[end + 1..].starts_with(' '))
                    });
                    let indent = prefix.len() - quote_width;
                    let expected_indent = lists.iter().rev().nth(1).map_or(0, |parent| {
                        if parent.ordered {
                            parent.content
                        } else {
                            parent.marker + indent_size
                        }
                    });
                    let Some(current) = lists.last_mut() else {
                        continue;
                    };
                    if !current.ordered && indent != expected_indent {
                        violations.push(Violation {
                            line: line_number, column: Some(1), rule: self.name().to_owned(),
                            message: format!("Unordered list indentation should be {expected_indent} spaces (found {indent})"),
                            fix: None,
                        });
                    }
                    let marker_width = item.chars().take_while(char::is_ascii_digit).count() + 1;
                    let spacing = item[marker_width..]
                        .chars()
                        .take_while(char::is_ascii_whitespace)
                        .count();
                    current.marker = indent;
                    current.content = indent + marker_width + spacing;
                }
                _ => {}
            }
        }

        violations
    }

    fn fixable(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lint::rules::rendered;
    use indoc::indoc;

    #[test]
    fn test_correct_indentation() {
        let content = indoc! {"
            * Item 1
              * Nested 1
                * Double nested
              * Nested 2
            * Item 2"};
        let parser = MarkdownParser::new(content);
        let rule = MD007;
        let violations = rule.check(&parser, None);

        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn ordered_parent_width_and_quote_prefix_are_not_unordered_indent() {
        for content in [
            "12. Parent\n    continuation\n    - child\n      continuation\n      - grandchild\n",
            "> 12. Parent\n>     - child\n",
        ] {
            assert!(
                MD007.check(&MarkdownParser::new(content), None).is_empty(),
                "{content}"
            );
        }
        assert!(
            !MD007
                .check(&MarkdownParser::new("> - parent\n>    - child\n"), None)
                .is_empty()
        );
    }

    #[test]
    fn test_incorrect_indentation() {
        let content = "* Item 1\n   * Nested wrong - 3 spaces instead of 2";
        let parser = MarkdownParser::new(content);
        let rule = MD007;
        let violations = rule.check(&parser, None);

        assert_eq!(
            rendered(&violations),
            ["test.md:2:1: MD007 Unordered list indentation should be 2 spaces (found 3)"]
        );
    }

    #[test]
    fn test_custom_indent_size() {
        let content = "* Item 1\n    * Nested with 4 spaces";
        let parser = MarkdownParser::new(content);
        let rule = MD007;
        let config = serde_json::json!({ "indent": 4 });
        let violations = rule.check(&parser, Some(&config));

        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_multiple_levels() {
        let content = indoc! {"
            * Level 1
              * Level 2
                * Level 3
                  * Level 4"};
        let parser = MarkdownParser::new(content);
        let rule = MD007;
        let violations = rule.check(&parser, None);

        assert_eq!(violations.len(), 0);
    }
}
