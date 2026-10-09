use crate::lint::rule::Rule;
use crate::markdown::MarkdownParser;
use crate::types::Violation;
use pulldown_cmark::{CodeBlockKind, Event, Tag};
use serde_json::Value;

pub struct MD048;

impl Rule for MD048 {
    fn name(&self) -> &'static str {
        "MD048"
    }

    fn description(&self) -> &'static str {
        "Code fence style"
    }

    fn tags(&self) -> &[&str] {
        &["code"]
    }

    fn check(&self, parser: &MarkdownParser, config: Option<&Value>) -> Vec<Violation> {
        let style = config
            .and_then(|c| c.get("style"))
            .and_then(|v| v.as_str())
            .unwrap_or("backtick");

        let mut violations = Vec::new();
        let mut first_style = None;
        for (event, range) in parser.parse_with_offsets() {
            let Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info))) = event else {
                continue;
            };
            let fence_char = parser.content()[range.clone()]
                .chars()
                .next()
                .unwrap_or('`');
            // Backticks in the info string require a tilde fence in CommonMark.
            if style == "backtick" && info.contains('`') {
                continue;
            }
            let message = match style {
                "consistent" => {
                    let first = *first_style.get_or_insert(fence_char);
                    (fence_char != first).then(|| format!(
                        "Code fence style should be consistent: expected '{first}', found '{fence_char}'"
                    ))
                }
                "tilde" if fence_char == '`' => {
                    Some("Code fence style should be 'tilde' (~), found backtick (`)".to_owned())
                }
                "backtick" if fence_char == '~' => {
                    Some("Code fence style should be 'backtick' (`), found tilde (~)".to_owned())
                }
                _ => None,
            };
            if let Some(message) = message {
                violations.push(Violation {
                    line: parser.offset_to_line(range.start),
                    column: Some(1),
                    rule: self.name().to_owned(),
                    message,
                    fix: None,
                });
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
    fn test_consistent_backtick() {
        let content = indoc! {"
            ```
            code1
            ```

            ```
            code2
            ```"};
        let parser = MarkdownParser::new(content);
        let rule = MD048;
        let violations = rule.check(&parser, None);

        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_consistent_tilde() {
        let content = indoc! {"
            ~~~
            code1
            ~~~

            ~~~
            code2
            ~~~"};
        let parser = MarkdownParser::new(content);
        let rule = MD048;
        let config = serde_json::json!({ "style": "consistent" });
        let violations = rule.check(&parser, Some(&config));

        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_inconsistent() {
        let content = indoc! {"
            ```
            code1
            ```

            ~~~
            code2
            ~~~"};
        let parser = MarkdownParser::new(content);
        let rule = MD048;
        let violations = rule.check(&parser, None);

        // Only the opening fence of the offending block is reported.
        assert_eq!(
            rendered(&violations),
            ["test.md:5:1: MD048 Code fence style should be 'backtick' (`), found tilde (~)"]
        );
    }

    #[test]
    fn test_enforced_backtick() {
        let content = indoc! {"
            ~~~
            code
            ~~~"};
        let parser = MarkdownParser::new(content);
        let rule = MD048;
        let config = serde_json::json!({ "style": "backtick" });
        let violations = rule.check(&parser, Some(&config));

        // Only the opening fence is reported, not the closing one.
        assert_eq!(
            rendered(&violations),
            ["test.md:1:1: MD048 Code fence style should be 'backtick' (`), found tilde (~)"]
        );
    }
}
