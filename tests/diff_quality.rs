use mdlint::config::Config;
use mdlint::fix::Fixer;
use mdlint::formatter;
use mdlint::lint::LintEngine;

fn assert_formats_to(input: &str, expected: &str) {
    assert_eq!(formatter::format(input), expected, "{input}");
    assert_eq!(
        formatter::format(expected),
        expected,
        "idempotency: {expected}"
    );
}

#[test]
fn sentence_edits_do_not_reflow_other_sentences() {
    assert_formats_to(
        "First sentence. Last sentence.\n",
        "First sentence.\nLast sentence.\n",
    );
    assert_formats_to(
        "First sentence. Inserted sentence. Last sentence.\n",
        "First sentence.\nInserted sentence.\nLast sentence.\n",
    );
    assert_formats_to(
        "A much longer first sentence without a width limit. Last sentence.\n",
        "A much longer first sentence without a width limit.\nLast sentence.\n",
    );
}

#[test]
fn sentence_boundaries_respect_markdown_context() {
    for (input, expected) in [
        ("First. Next! Last?\n", "First.\nNext!\nLast?\n"),
        (
            "First sentence,\nwith a clause. Next sentence.\n",
            "First sentence,\nwith a clause.\nNext sentence.\n",
        ),
        ("First. Next.  \nLast.\n", "First.\nNext.\\\nLast.\n"),
        ("- First. Next.\n", "- First.\n  Next.\n"),
        ("7. First. Next.\n", "7. First.\n   Next.\n"),
        ("> First. Next.\n", "> First.\n> Next.\n"),
        (
            "12. First. Next.\n    - Child. Again.\n",
            "12. First.\n    Next.\n    - Child.\n      Again.\n",
        ),
        (
            "Élan starts here. Über text follows.\n",
            "Élan starts here.\nÜber text follows.\n",
        ),
        (
            "text[^n]\n\n[^n]: First. Next.\n",
            "text[^n]\n\n[^n]: First.\n    Next.\n",
        ),
        (
            "Read [the guide](https://example.com). Next sentence.\n",
            "Read [the guide](https://example.com).\nNext sentence.\n",
        ),
        (
            "First. \"Next sentence.\" Last sentence.\n",
            "First.\n\"Next sentence.\"\nLast sentence.\n",
        ),
    ] {
        assert_formats_to(input, expected);
    }
}

#[test]
fn ambiguous_and_protected_sentence_text_is_unchanged() {
    for input in [
        "Ask Dr. Smith and Prof. Jones about Fig. One.\n",
        "Use e.g. Rust and i.e. Examples with U.S. Names.\n",
        "Version 1.2. Next is README.md. Another filename.\n",
        "A. Smith wrote it... Perhaps.\n",
        "A sentence. lowercase text remains here.\n",
        "# First. Next.\n",
        "| First. Next. |\n| --- |\n| Value. Next. |\n",
        "```text\nFirst. Next.\n```\n",
        "Use `First. Next.` here.\n",
        "Use **First. Next.** here.\n",
        "[First. Next.](https://example.com \"First. Next.\")\n",
        "[First. Next.][ref]\n\n[ref]: /example\n",
        "[First. Next.]: /example\n\nRead [First. Next.].\n",
        "Text <span>First. Next.</span> text.\n",
    ] {
        assert_formats_to(input, input);
    }
}

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
