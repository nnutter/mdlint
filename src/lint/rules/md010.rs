use crate::lint::rule::Rule;
use crate::markdown::MarkdownParser;
use crate::types::{Fix, Violation};
use serde_json::Value;

pub struct MD010;

impl Rule for MD010 {
    fn name(&self) -> &'static str {
        "MD010"
    }

    fn description(&self) -> &'static str {
        "Hard tabs"
    }

    fn tags(&self) -> &[&str] {
        &["whitespace", "hard_tab"]
    }

    fn check(&self, parser: &MarkdownParser, config: Option<&Value>) -> Vec<Violation> {
        let code_blocks = config
            .and_then(|c| c.get("code_blocks"))
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);

        let mut violations = Vec::new();
        let code_block_lines = parser.get_code_block_line_numbers();

        for (line_num, line) in parser.lines().iter().enumerate() {
            let line_number = line_num + 1;

            // Skip code blocks if configured
            if !code_blocks && code_block_lines.contains(&line_number) {
                continue;
            }

            if let Some(tab_pos) = line.find('\t') {
                violations.push(Violation {
                    line: line_number,
                    column: Some(tab_pos + 1),
                    rule: self.name().to_owned(),
                    message: "Hard tabs found".to_owned(),
                    fix: Some(Fix {
                        line_start: line_number,
                        line_end: line_number,
                        column_start: None,
                        column_end: None,
                        replacement: line.replace('\t', "    "),
                        description: "Replace tabs with spaces".to_owned(),
                    }),
                });
            }
        }

        violations
    }

    fn fixable(&self) -> bool {
        true
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
    fn test_no_tabs() {
        let content = indoc! {"
            Line 1
                Line 2
            Line 3"};
        let parser = MarkdownParser::new(content);
        let rule = MD010;
        let violations = rule.check(&parser, None);

        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_hard_tabs() {
        let content = indoc! {"
            Line 1
            Text\tLine 2
            Line 3"};
        let parser = MarkdownParser::new(content);
        let rule = MD010;
        let violations = rule.check(&parser, None);

        assert_eq!(
            rendered(&violations),
            ["test.md:2:5: MD010 Hard tabs found"]
        );
    }

    #[test]
    fn test_tabs_in_code_block() {
        let content = indoc! {"
            Text
            ```
            \tcode
            ```"};
        let parser = MarkdownParser::new(content);
        let rule = MD010;
        let violations = rule.check(&parser, None);

        assert!(violations.is_empty());
        let config = serde_json::json!({"code_blocks": true});
        assert_eq!(
            rendered(&rule.check(&parser, Some(&config))),
            ["test.md:3:1: MD010 Hard tabs found"]
        );
    }

    #[test]
    fn test_ignore_code_blocks() {
        let content = indoc! {"
            Text
            ```
            \tcode
            ```"};
        let parser = MarkdownParser::new(content);
        let rule = MD010;
        let config = serde_json::json!({ "code_blocks": false });
        let violations = rule.check(&parser, Some(&config));

        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn ignoring_code_blocks_skips_every_content_line() {
        let config = serde_json::json!({"code_blocks": false});
        let parser = MarkdownParser::new("```text\nfirst\nsecond\tline\nthird\tline\n```\n");
        assert!(MD010.check(&parser, Some(&config)).is_empty());
    }

    #[test]
    fn test_fix_replaces_tab_with_spaces() {
        let content = indoc! {"
            # Heading

            Text\tTabbed line
        "};
        let parser = MarkdownParser::new(content);
        let rule = MD010;
        let violations = rule.check(&parser, None);
        let fixed = apply_fixes(content, &violations);
        assert!(!fixed.contains('\t'), "tabs should be replaced after fix");
        assert!(
            fixed.contains("    Tabbed line"),
            "tab should become 4 spaces"
        );
    }
}
