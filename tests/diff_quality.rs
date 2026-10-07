use mdlint::config::Config;
use mdlint::fix::Fixer;
use mdlint::lint::LintEngine;

#[test]
fn whitespace_fixes_leave_code_examples_unchanged() {
    let code = "```text\nline   \n\n\n\t\n```\n";
    let input = format!("Text   \n\n\n{code}");
    let expected = format!("Text\n\n{code}");
    let config =
        Config::default().apply_rule_filters(&["MD009".to_owned(), "MD012".to_owned()], &[]);
    let engine = LintEngine::new(config);
    let violations = engine.lint_content(&input).unwrap();
    assert!(
        violations.iter().all(|v| v.line < 4),
        "code violations: {violations:?}"
    );
    let fixes: Vec<_> = violations.iter().filter_map(|v| v.fix.clone()).collect();
    assert_eq!(
        Fixer::new().apply_fixes_to_content(&input, &fixes).unwrap(),
        expected
    );
    assert!(engine.lint_content(&expected).unwrap().is_empty());
}
