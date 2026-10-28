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
fn sentences_are_split_and_wrapped_lines_are_joined() {
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
    assert_formats_to(
        "A wrapped\nsentence continues here.\n",
        "A wrapped sentence continues here.\n",
    );
    assert_formats_to(
        "A longer sentence with a random\nnewline in the middle of it.",
        "A longer sentence with a random newline in the middle of it.\n",
    );
    assert_formats_to(
        "mdlint is configured. A wrapped\nsentence continues here.\n",
        "mdlint is configured.\nA wrapped sentence continues here.\n",
    );
    for input in ["A\ra\ra", "A\na\na", "A\r\na\r\na"] {
        assert_formats_to(input, "A a a\n");
    }
}

#[test]
fn wrapped_lines_join_across_inline_markdown() {
    assert_formats_to(
        r#"- Settings can also live in a manifest: `[tool.mdlint]` in `pyproject.toml`, `"mdlint"` in `package.json`
  (`src/config/embedded.rs`). A manifest with no mdlint section returns `Ok(None)` so discovery keeps
  walking up instead of stopping there; `--config` pointed at such a manifest is an error
"#,
        r#"- Settings can also live in a manifest: `[tool.mdlint]` in `pyproject.toml`, `"mdlint"` in `package.json` (`src/config/embedded.rs`).
  A manifest with no mdlint section returns `Ok(None)` so discovery keeps walking up instead of stopping there; `--config` pointed at such a manifest is an error
"#,
    );
    assert_formats_to(
        r"- **Scalars** — `default_enabled`, `fix`, `gitignore`, `no_inline_config`, `front_matter`: the nearest config that
  *sets* the option wins. A config that omits an option leaves whatever an outer config set it to, rather than resetting it to the built-in default.
",
        r"- **Scalars** — `default_enabled`, `fix`, `gitignore`, `no_inline_config`, `front_matter`: the nearest config that *sets* the option wins.
  A config that omits an option leaves whatever an outer config set it to, rather than resetting it to the built-in default.
",
    );
    assert_formats_to(
        "`capture`/`restore` nest like a stack, so a block of directives can be undone as a unit regardless of what state it\nstarted from. Directives shown inside a code span or code block are examples and have no effect.\n",
        "`capture`/`restore` nest like a stack, so a block of directives can be undone as a unit regardless of what state it started from.\nDirectives shown inside a code span or code block are examples and have no effect.\n",
    );
}

#[test]
fn sentence_boundaries_and_hard_breaks_stay_on_separate_lines() {
    for (input, expected) in [
        ("First. Next! Last?\n", "First.\nNext!\nLast?\n"),
        (
            "First sentence,\nwith a clause. Next sentence.\n",
            "First sentence, with a clause.\nNext sentence.\n",
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
        (
            "First. (Next sentence.) Last one!\n",
            "First.\n(Next sentence.)\nLast one!\n",
        ),
        (
            "- A wrapped\n  item continues.\n",
            "- A wrapped item continues.\n",
        ),
        (
            "- keep this line\n  because it does not start a sentence.\n",
            "- keep this line\n  because it does not start a sentence.\n",
        ),
        (
            "> A wrapped\n> quote continues.\n",
            "> A wrapped quote continues.\n",
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
