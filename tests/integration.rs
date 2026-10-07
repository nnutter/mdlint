use indoc::indoc;
use mdlint::config::{Config, RuleConfig};
use mdlint::fix::Fixer;
use mdlint::formatter;
use mdlint::lint::LintEngine;
use std::fs;

// ── Helpers ──────────────────────────────────────────────────────────────────

fn fixture(path: &str) -> String {
    let full = format!("tests/fixtures/{path}");
    fs::read_to_string(&full).unwrap_or_else(|e| panic!("cannot read fixture {full}: {e}"))
}

fn all_rules_engine() -> LintEngine {
    LintEngine::new(Config::default().apply_rule_filters(&["ALL".to_owned()], &[]))
}

// ── Format workflow tests ─────────────────────────────────────────────────────

#[test]
fn format_produces_expected_output() {
    let input = fixture("format/input.md");
    let expected = fixture("format/expected.md");
    let got = formatter::format(&input);
    assert_eq!(got, expected, "formatter output did not match golden file");
}

#[test]
fn format_is_idempotent_on_expected() {
    let expected = fixture("format/expected.md");
    let twice = formatter::format(&expected);
    assert_eq!(
        expected, twice,
        "formatting the already-formatted file produced different output"
    );
}

#[test]
fn format_empty_string_is_empty() {
    assert_eq!(formatter::format(""), "");
}

#[test]
fn format_whitespace_only_is_empty() {
    assert_eq!(formatter::format("   \n\n  \t\n"), "");
}

// ── Check workflow tests ──────────────────────────────────────────────────────

#[test]
fn check_detects_heading_level_skip() {
    let content = fixture("check/violations.md");
    let engine = all_rules_engine();
    let violations = engine.lint_content(&content).unwrap();
    assert!(
        violations.iter().any(|v| v.rule == "MD001"),
        "MD001 (heading level skip) should fire; got: {violations:?}"
    );
}

#[test]
fn check_detects_trailing_spaces() {
    // Use inline content so trailing spaces aren't stripped by the editor
    let content = "# Heading\n\nSome text   \nMore text\n";
    let engine = all_rules_engine();
    let violations = engine.lint_content(content).unwrap();
    assert!(
        violations.iter().any(|v| v.rule == "MD009"),
        "MD009 (trailing spaces) should fire; got: {violations:?}"
    );
}

#[test]
fn check_fix_removes_trailing_spaces() {
    let content = "# Heading\n\nSome text   \nMore text\n";
    let engine = all_rules_engine();
    let violations = engine.lint_content(content).unwrap();
    let fixes: Vec<_> = violations.iter().filter_map(|v| v.fix.clone()).collect();
    assert!(!fixes.is_empty(), "MD009 should produce an inline fix");
    let result = Fixer::new()
        .apply_fixes_to_content(content, &fixes)
        .unwrap();
    assert_eq!(
        result,
        indoc! {"
            # Heading

            Some text
            More text
        "}
    );
}

#[test]
fn default_checks_do_not_enforce_optional_document_policies() {
    let content = format!(
        "## Why!\n\n<div>HTML</div>\n\n{}\n",
        "Long prose ".repeat(30)
    );
    for config in [
        Config::default(),
        toml::from_str::<Config>("fix = false").unwrap(),
    ] {
        let violations = LintEngine::new(config).lint_content(&content).unwrap();
        assert!(violations.iter().all(|v| !matches!(
            v.rule.as_str(),
            "MD013" | "MD026" | "MD033" | "MD041" | "MD043"
        )));
    }
    let selected = Config::default().apply_rule_filters(&["ALL".to_owned()], &[]);
    assert!(
        LintEngine::new(selected)
            .lint_content(&content)
            .unwrap()
            .iter()
            .any(|v| v.rule == "MD013")
    );
}

#[test]
fn default_list_indentation_check_accepts_wide_ordered_parents() {
    let content =
        mdlint::formatter::format("# Title\n\n12. First. Next.\n    - Child. Again.\n\n1. Last.\n");
    let violations = LintEngine::new(Config::default())
        .lint_content(&content)
        .unwrap();
    assert!(violations.is_empty(), "{content}\n{violations:?}");
}

#[test]
fn emphasis_fixes_do_not_change_reference_identities() {
    for (style, source, target) in [("asterisk", "_", "*"), ("underscore", "*", "_")] {
        let strong = source.repeat(2);
        let input = format!(
            "[foo {source}bar{source}]: /guide\n[foo {strong}bar{strong}]: /strong\n\nRead [foo {source}bar{source}] and [foo {strong}bar{strong}][] and {source}prose{source}.\n"
        );
        let expected = input.replace(
            &format!("{source}prose{source}"),
            &format!("{target}prose{target}"),
        );
        let config: Config = toml::from_str(&format!("default_enabled = false\n[rules.MD049]\nstyle = \"{style}\"\n[rules.MD050]\nstyle = \"{style}\"\n")).unwrap();
        let engine = LintEngine::new(config);
        let fixes: Vec<_> = engine
            .lint_content(&input)
            .unwrap()
            .iter()
            .filter_map(|v| v.fix.clone())
            .collect();
        let fixed = Fixer::new().apply_fixes_to_content(&input, &fixes).unwrap();
        assert_eq!(fixed, expected);
        assert!(engine.lint_content(&fixed).unwrap().is_empty());
        if style == "asterisk" {
            assert_eq!(mdlint::formatter::format(&input), expected);
        }
    }
}

#[test]
fn optional_policies_still_accept_explicit_configuration() {
    for (rule, content, settings) in [
        (
            "MD013",
            "A sentence longer than the configured limit.\n",
            "line_length = 10",
        ),
        ("MD026", "# Why!\n", "enabled = true"),
        ("MD033", "<div>HTML</div>\n", "enabled = true"),
        ("MD041", "A document fragment.\n", "enabled = true"),
        ("MD043", "# Actual\n", "headings = [\"Required\"]"),
    ] {
        let config: Config = toml::from_str(&format!(
            "default_enabled = false\n[rules.{rule}]\n{settings}\n"
        ))
        .unwrap();
        assert!(
            LintEngine::new(config)
                .lint_content(content)
                .unwrap()
                .iter()
                .any(|v| v.rule == rule),
            "{rule}"
        );
    }
}

#[test]
fn default_checks_allow_code_tabs_and_headings_in_separate_sections() {
    let config =
        Config::default().apply_rule_filters(&["MD010".to_owned(), "MD024".to_owned()], &[]);
    let engine = LintEngine::new(config);
    let content =
        "## One\n\n### Examples\n\n## Two\n\n### Examples\n\n```text\nfirst\nsecond\tline\n```\n";
    assert!(engine.lint_content(content).unwrap().is_empty());
    assert!(
        engine
            .lint_content("Text\twith a tab\n")
            .unwrap()
            .iter()
            .any(|v| v.rule == "MD010")
    );
}

#[test]
fn ordered_list_fixes_preserve_starts_and_match_formatting() {
    let config = Config::default().apply_rule_filters(&["MD029".to_owned()], &[]);
    let engine = LintEngine::new(config);
    for (input, expected) in [
        ("7. first\n8. second\n", "7. first\n1. second\n"),
        ("> 7. first\n> 8. second\n", "> 7. first\n> 1. second\n"),
        (
            "- parent\n\n  7. first\n  8. second\n",
            "- parent\n\n  7. first\n  1. second\n",
        ),
    ] {
        let violations = engine.lint_content(input).unwrap();
        let fixes: Vec<_> = violations.iter().filter_map(|v| v.fix.clone()).collect();
        assert_eq!(
            Fixer::new().apply_fixes_to_content(input, &fixes).unwrap(),
            expected
        );
        assert_eq!(formatter::format(input), expected);
        assert!(engine.lint_content(expected).unwrap().is_empty());
    }
}

#[test]
fn safe_formatter_delimiters_pass_style_checks() {
    let config = Config::default().apply_rule_filters(
        &["MD048".to_owned(), "MD049".to_owned(), "MD050".to_owned()],
        &[],
    );
    let engine = LintEngine::new(config);
    for input in ["~~~lang`tag\ncode\n~~~\n", "_one_*two*\n", "_one_**two**\n"] {
        let formatted = formatter::format(input);
        assert!(
            engine.lint_content(&formatted).unwrap().is_empty(),
            "{formatted}"
        );
        assert_eq!(formatter::format(&formatted), formatted);
        let violations = engine.lint_content(input).unwrap();
        let fixes: Vec<_> = violations.iter().filter_map(|v| v.fix.clone()).collect();
        if !input.starts_with("~~~") {
            assert_eq!(
                Fixer::new().apply_fixes_to_content(input, &fixes).unwrap(),
                formatted
            );
        }
    }
}

#[test]
fn unicode_line_length_diagnostic_uses_a_byte_column() {
    let config = Config {
        default_enabled: false,
        rules: std::collections::HashMap::from([(
            "MD013".to_owned(),
            RuleConfig::Config(std::collections::HashMap::from([(
                "line_length".to_owned(),
                toml::Value::Integer(2),
            )])),
        )]),
        ..Config::default()
    };
    let violations = LintEngine::new(config).lint_content("é😀x\n").unwrap();
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].column, Some(7));
}

#[test]
fn unicode_inline_fixes_use_byte_columns() {
    for (rule, input, expected, columns) in [
        ("MD009", "é😀   \n", "é😀\n", vec![7]),
        ("MD049", "é _😀_\n", "é *😀*\n", vec![4, 9]),
        ("MD050", "é __😀__\n", "é **😀**\n", vec![4, 10]),
    ] {
        let config = Config::default().apply_rule_filters(&[rule.to_owned()], &[]);
        let engine = LintEngine::new(config);
        let violations = engine.lint_content(input).unwrap();
        let mut actual_columns: Vec<_> = violations.iter().map(|v| v.column.unwrap()).collect();
        actual_columns.sort_unstable();
        assert_eq!(actual_columns, columns, "{rule}");
        let fixes: Vec<_> = violations.into_iter().map(|v| v.fix.unwrap()).collect();
        let result = Fixer::new().apply_fixes_to_content(input, &fixes).unwrap();
        assert_eq!(result, expected, "{rule}");
        assert!(engine.lint_content(expected).unwrap().is_empty(), "{rule}");
    }
}

#[test]
fn check_detects_hard_tabs() {
    let content = fixture("check/violations.md");
    let engine = all_rules_engine();
    let violations = engine.lint_content(&content).unwrap();
    assert!(
        violations.iter().any(|v| v.rule == "MD010"),
        "MD010 (hard tabs) should fire; got: {violations:?}"
    );
}

#[test]
fn check_fix_replaces_hard_tabs() {
    let content = indoc! {"
        # Heading

        Text\tTabbed line
    "};
    let engine = all_rules_engine();
    let violations = engine.lint_content(content).unwrap();
    let fixes: Vec<_> = violations.iter().filter_map(|v| v.fix.clone()).collect();
    assert!(!fixes.is_empty(), "MD010 should produce an inline fix");
    let result = Fixer::new()
        .apply_fixes_to_content(content, &fixes)
        .unwrap();
    assert!(!result.contains('\t'), "tabs should be replaced after fix");
}

#[test]
fn format_output_for_nested_list_passes_check() {
    // Regression test for issue #67: `mdlint format`'s output for a bullet item
    // containing a nested ordered sub-list must pass `mdlint check` cleanly.
    // The formatter used to strip the blank lines around the nested list
    // (intended, canonical behavior), but MD032 then misjudged the nested,
    // differently-marked sub-list as a set of separate top-level lists,
    // reporting spurious violations on the formatter's own output.
    let content = indoc! {"
        # Example

        - First item:

          1. One
          2. Two

        - Second item
    "};
    let formatted = formatter::format(content);
    let engine = all_rules_engine();
    let violations = engine.lint_content(&formatted).unwrap();
    assert!(
        violations.is_empty(),
        "mdlint check should report no violations on mdlint format's output; got: {violations:?}\nformatted:\n{formatted}"
    );
}

#[test]
fn check_clean_file_has_no_violations() {
    let content = fixture("format/expected.md");
    let engine = LintEngine::new(Config {
        default_enabled: true,
        rules: {
            // MD041 requires a top-level heading — our fixture has one, but MD013
            // line length and other cosmetic rules might fire; only disable none.
            std::collections::HashMap::new()
        },
        ..Config::default()
    });
    let violations = engine.lint_content(&content).unwrap();
    // Only allow violations that are expected (none for a well-formed file)
    // Filter out MD041 if the expected.md doesn't start with a top-level heading
    let non_trivial: Vec<_> = violations
        .iter()
        .filter(|v| !matches!(v.rule.as_str(), "MD013" | "MD043"))
        .collect();
    assert!(
        non_trivial.is_empty(),
        "formatted file should have no violations (except MD013/MD043): {non_trivial:?}"
    );
}

// ── --select / --ignore filtering (issue #68) ──────────────────────────────────

#[test]
fn select_restricts_check_to_named_rule() {
    // Line has both a heading-level skip (MD001) and a prose tab (MD010).
    let content = indoc! {"
        # Heading

        ### Skipped level

        Text\tTabbed line
    "};
    let config = Config::default().apply_rule_filters(&["MD001".to_owned()], &[]);
    let violations = LintEngine::new(config).lint_content(content).unwrap();
    assert!(violations.iter().any(|v| v.rule == "MD001"));
    assert!(violations.iter().all(|v| v.rule == "MD001"));
}

#[test]
fn ignore_excludes_named_rule_only() {
    let content = indoc! {"
        # Heading

        ### Skipped level

        Text\tTabbed line
    "};
    let config = Config::default().apply_rule_filters(&[], &["MD010".to_owned()]);
    let violations = LintEngine::new(config).lint_content(content).unwrap();
    assert!(violations.iter().any(|v| v.rule == "MD001"));
    assert!(violations.iter().all(|v| v.rule != "MD010"));
}

#[test]
fn ignore_overrides_config_file_enabling_the_rule() {
    let content = "Text\tTabbed line\n";
    let mut config = Config::default();
    config
        .rules
        .insert("MD010".to_owned(), RuleConfig::Enabled(true));
    let config = config.apply_rule_filters(&[], &["MD010".to_owned()]);
    let violations = LintEngine::new(config).lint_content(content).unwrap();
    assert!(violations.iter().all(|v| v.rule != "MD010"));
}
