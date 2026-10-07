use indoc::indoc;
use mdlint::formatter;
use std::fs;
use std::process::{Command, Stdio};
use tempfile::TempDir;

// ── helpers ──────────────────────────────────────────────────────────────────

/// Assert that formatting `input` produces `expected`, and that the expected
/// output is already idempotent (format(expected) == expected).
fn assert_formats_to(input: &str, expected: &str) {
    let got = formatter::format(input);
    assert_eq!(
        got, expected,
        "format(input) did not match expected.\nInput:\n{input}\nExpected:\n{expected}\nGot:\n{got}"
    );
    let twice = formatter::format(expected);
    assert_eq!(
        twice, expected,
        "format(expected) != expected — expected output is not idempotent.\nExpected:\n{expected}\nTwice:\n{twice}"
    );
}

fn mdlint_bin() -> std::path::PathBuf {
    // Use the debug build so tests don't need a release build.
    let mut p = std::env::current_exe().unwrap();
    p.pop(); // remove test binary name
    if p.ends_with("deps") {
        p.pop();
    }
    p.push("mdlint");
    p
}

// ── canonicalization ─────────────────────────────────────────────────────────

#[test]
fn setext_headings_become_atx() {
    assert_formats_to(
        indoc! {"
            Title
            =====

            Section
            -------
        "},
        indoc! {"
            # Title

            ## Section
        "},
    );
}

#[test]
fn closed_atx_headings_stripped() {
    assert_formats_to(
        indoc! {"
            ## Heading ##

            ### Sub ###
        "},
        indoc! {"
            ## Heading

            ### Sub
        "},
    );
}

#[test]
fn extra_spaces_after_hash_collapsed() {
    assert_formats_to(
        indoc! {"
            #  Too many

            ###   Lots
        "},
        indoc! {"
            # Too many

            ### Lots
        "},
    );
}

#[test]
fn asterisk_and_plus_list_markers_become_dash() {
    // Two lists with different markers both normalise to `-`.  An invisible
    // HTML comment separator is inserted so they don't merge into a single
    // list (and become loose) on the second format pass.
    assert_formats_to(
        indoc! {"
            * Alpha
            * Beta

            + Gamma
            + Delta
        "},
        indoc! {"
            - Alpha
            - Beta

            <!---->

            - Gamma
            - Delta
        "},
    );
}

#[test]
fn tilde_code_fences_become_backtick() {
    assert_formats_to(
        indoc! {"
            ~~~rust
            fn main() {}
            ~~~
        "},
        indoc! {"
            ```rust
            fn main() {}
            ```
        "},
    );
    assert_formats_to(
        indoc! {"
            ~~~
            plain
            ~~~
        "},
        indoc! {"
            ```
            plain
            ```
        "},
    );
}

#[test]
fn underscore_emphasis_becomes_asterisk() {
    assert_formats_to("_italic_ and __bold__\n", "*italic* and **bold**\n");
}

#[test]
fn horizontal_rules_normalised_to_dashes() {
    assert_formats_to("***\n", "---\n");
    assert_formats_to("___\n", "---\n");
    assert_formats_to("* * *\n", "---\n");
    assert_formats_to("- - -\n", "---\n");
}

#[test]
fn multiple_blank_lines_collapsed() {
    assert_formats_to(
        indoc! {"
            First.



            Second.
        "},
        indoc! {"
            First.

            Second.
        "},
    );
}

#[test]
fn trailing_whitespace_removed() {
    // Lines with trailing spaces get stripped
    let input = "Text with trailing spaces.   \n\nMore text.  \n";
    let out = formatter::format(input);
    for line in out.lines() {
        assert_eq!(
            line,
            line.trim_end(),
            "line has trailing whitespace: {line:?}"
        );
    }
}

#[test]
fn trailing_newline_normalised() {
    assert!(formatter::format("text").ends_with('\n'));
    assert!(
        formatter::format(indoc! {"
            text


        "})
        .ends_with('\n')
    );
    assert_eq!(
        formatter::format(indoc! {"
            text


        "})
        .matches('\n')
        .count(),
        1
    );
}

#[test]
fn empty_input_produces_empty_output() {
    assert_eq!(formatter::format(""), "");
    assert_eq!(formatter::format("   \n\n  "), "");
}

// ── structure preservation ────────────────────────────────────────────────────

#[test]
fn nested_lists_preserved() {
    assert_formats_to(
        indoc! {"
            - Top
              - Nested
                - Deep
            - Back
        "},
        indoc! {"
            - Top
              - Nested
                - Deep
            - Back
        "},
    );
}

#[test]
fn nested_ordered_list_preserves_loose_parent_spacing() {
    assert_formats_to(
        indoc! {"
            # Example

            - First item:

              1. One
              2. Two

            - Second item
        "},
        indoc! {"
            # Example

            - First item:
              1. One
              1. Two

            - Second item
        "},
    );
}

#[test]
fn ordered_lists_use_stable_markers_and_preserve_the_start() {
    assert_formats_to(
        "7. first\n8. second\n9. third\n",
        "7. first\n1. second\n1. third\n",
    );
    let before = "1. first\n1. last\n";
    let after = "1. first\n1. inserted\n1. last\n";
    assert_formats_to(before, before);
    assert_formats_to(after, after);
}

#[test]
fn ordered_marker_width_controls_nested_list_indentation() {
    assert_formats_to(
        "12. parent\n    - child\n      continuation\n",
        "12. parent\n    - child\n      continuation\n",
    );
}

#[test]
fn paragraphs_after_nested_lists_stay_in_the_parent_item() {
    assert_formats_to(
        "12. parent\n    - child\n\n    after\n",
        "12. parent\n    - child\n\n    after\n",
    );
}

#[test]
fn loose_list_item_spacing_survives_nested_lists() {
    assert_formats_to(
        "12. parent\n    - child\n\n1. next\n",
        "12. parent\n    - child\n\n1. next\n",
    );
}

#[test]
fn ordered_list_preserved() {
    let input = "1. First\n1. Second\n1. Third\n";
    assert_formats_to(input, input);
}

#[test]
fn list_item_paragraphs_keep_their_boundaries_and_indentation() {
    assert_formats_to(
        "* é\n  + nested\n* second\n\n    a\n",
        "- é\n  - nested\n\n- second\n\n  a\n",
    );
    assert_formats_to("1. first\n\n   second\n", "1. first\n\n   second\n");
}

#[test]
fn footnote_paragraphs_keep_their_indentation() {
    assert_formats_to(
        "é[^note]\n\n[^note]: body\n\n    😀\n",
        "é[^note]\n\n[^note]: body\n\n    😀\n",
    );
    assert_formats_to(
        "note[^n]\n\n[^n]: first\n    second\n",
        "note[^n]\n\n[^n]: first\n    second\n",
    );
}

#[test]
fn code_block_content_preserved_verbatim() {
    // Tabs and unusual indentation inside code blocks must survive unchanged.
    let input = indoc! {"
        ```
        \tindented with tab
            four spaces
        ```
    "};
    assert_formats_to(input, input);
}

#[test]
fn code_whitespace_is_not_prose_whitespace() {
    for input in [
        "```text\nline  \n\n\n\t\nlast\t\n```\n",
        "- item\n\n  ```text\n  line  \n  \n  \n  last\t\n  ```\n",
        "> ```text\n> line  \n> \n> \n> last\t\n> ```\n",
    ] {
        assert_formats_to(input, input);
    }
}

#[test]
fn code_fences_cannot_close_on_their_own_content() {
    for (input, expected) in [
        ("~~~rust\n```\n~~~\n", "````rust\n```\n````\n"),
        ("~~~rust\n````\n~~~\n", "`````rust\n````\n`````\n"),
        ("~~~lang`tag\ncode\n~~~\n", "~~~lang`tag\ncode\n~~~\n"),
        ("~~~~lang`tag\n~~~\n~~~~\n", "~~~~lang`tag\n~~~\n~~~~\n"),
        ("> ~~~rust\n> ```\n> ~~~\n", "> ````rust\n> ```\n> ````\n"),
        (
            "- item\n\n  ~~~rust\n  ```\n  ~~~\n",
            "- item\n\n  ````rust\n  ```\n  ````\n",
        ),
    ] {
        assert_formats_to(input, expected);
    }
}

#[test]
fn inline_code_content_preserved() {
    assert_formats_to(
        "Use `_underscores_` and `* asterisks` in code spans.\n",
        "Use `_underscores_` and `* asterisks` in code spans.\n",
    );
}

#[test]
fn adjacent_emphasis_keeps_distinct_delimiters() {
    assert_formats_to("_¡_*0*\n", "*¡*_0_\n");
    assert_formats_to("_one_*two*\n", "*one*_two_\n");
}

#[test]
fn literal_asterisks_do_not_become_emphasis() {
    for (input, expected) in [
        ("é \\*literal\\*\n", "é \\*literal\\*\n"),
        ("é \\*\\*literal\\*\\*\n", "é \\*\\*literal\\*\\*\n"),
        ("_\\*literal\\*_\n", "*\\*literal\\**\n"),
    ] {
        assert_formats_to(input, expected);
    }
}

#[test]
fn reference_links_keep_their_source_labels_and_definition_order() {
    for input in [
        "Read [the guide][Guide Name].\n\n[Guide Name]: https://example.com/install \"Guide\"\n",
        "[guide]: https://example.com\n\nRead [guide][] and [guide].\n",
        "![image][Picture]\n\n[unused]: /unused\n[Picture]: image.png\n",
        "First paragraph.\n\n[guide]: /guide\n\nRead [guide].\n",
        "[guide]: /guide\n  \"Multiline title\"\n",
        "> [guide]: /guide\n>\n> Read [guide].\n",
        "- Read [guide].\n\n  [guide]: /guide\n",
        "[guide]: /first\n[guide]: /second\n\nRead [guide].\n",
        "[foo _bar_]: /guide\n\nRead [foo _bar_][] and [foo _bar_].\n",
        "[a\\[b]: /guide\n\nRead [text][a\\[b].\n",
        "- [guide]: /guide\n\nRead [guide].\n",
    ] {
        assert_formats_to(input, input);
    }
    let before = "Read [guide].\n\n[guide]: /old\n";
    let after = "Read [guide].\n\n[guide]: /new\n";
    assert_formats_to(before, before);
    assert_formats_to(after, after);
}

#[test]
fn link_and_image_preserved() {
    assert_formats_to(
        "[link](https://example.com) and ![img](pic.png)\n",
        "[link](https://example.com) and ![img](pic.png)\n",
    );
}

#[test]
fn blockquote_preserved() {
    assert_formats_to(
        indoc! {"
            > quoted
            >
            > second para
        "},
        indoc! {"
            > quoted
            >
            > second para
        "},
    );
}

#[test]
fn definition_like_paragraph_text_does_not_create_references() {
    for input in [
        "Text\n[guide]: /example\n\nRead [guide].\n",
        "Text\n  [guide]: /example\n\nRead [guide].\n",
    ] {
        let expected = "Text\n\\[guide]: /example\n\nRead [guide].\n";
        assert_formats_to(input, expected);
    }
}

#[test]
fn changing_a_table_cell_does_not_resize_other_rows() {
    let before = "| Name | Count |\n| :--- | ---: |\n| short | 1 |\n| other | 2 |\n";
    let after = "| Name | Count |\n| :--- | ---: |\n| a much longer name | 1 |\n| other | 2 |\n";
    assert_formats_to(before, before);
    assert_formats_to(after, after);
    assert_formats_to(
        "| Name  | Count |\n| :----- | -----: |\n| short | 1     |\n| other | 2     |\n",
        before,
    );
}

#[test]
fn gfm_table_canonicalised() {
    // Input without leading/trailing pipes → output with them
    assert_formats_to(
        indoc! {"
            A | B
            --- | ---
            1 | 2
        "},
        indoc! {"
            | A | B |
            | --- | --- |
            | 1 | 2 |
        "},
    );
}

#[test]
fn gfm_table_already_canonical_unchanged() {
    let canonical = indoc! {"
        | A | B |
        | --- | --- |
        | 1 | 2 |
    "};
    assert_formats_to(canonical, canonical);
}

#[test]
fn list_item_continuation_indented() {
    // A soft-wrapped list item must keep its continuation indented so the
    // linter does not mistake it for a paragraph outside the list.
    assert_formats_to(
        indoc! {"
            - First line
              continuation here
        "},
        indoc! {"
            - First line
              continuation here
        "},
    );
}

// ── idempotency on complex documents ─────────────────────────────────────────

#[test]
fn idempotent_on_mixed_document() {
    let input = indoc! {"
        # Title

        Intro paragraph.

        ## Section

        - Item one
        - Item two
          - Nested

        ```rust
        fn main() {}
        ```

        | Col A | Col B |
        | ----- | ----- |
        | val   | val   |

        > A blockquote

        Final paragraph.
    "};
    let once = formatter::format(input);
    let twice = formatter::format(&once);
    assert_eq!(once, twice, "formatter is not idempotent on mixed document");
}

// ── `mdlint format` CLI ──────────────────────────────────────────────────────

#[test]
fn format_check_does_not_modify_file() {
    // `format --check` must never write to disk even when changes are needed.
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("doc.md");
    let original = indoc! {"
        Heading
        =======

        * item
    "};
    fs::write(&file, original).unwrap();

    Command::new(mdlint_bin())
        .args(["format", "--check", file.to_str().unwrap()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();

    let after = fs::read_to_string(&file).unwrap();
    assert_eq!(after, original, "format --check must not modify the file");
}

#[test]
fn format_check_exits_0_when_already_formatted() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("clean.md");
    fs::write(
        &file,
        indoc! {"
            # Heading

            Paragraph.
        "},
    )
    .unwrap();

    let status = Command::new(mdlint_bin())
        .args(["format", "--check", file.to_str().unwrap()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();

    assert!(
        status.success(),
        "expected exit 0 for already-formatted file"
    );
}

#[test]
fn format_check_exits_1_when_file_needs_formatting() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("dirty.md");
    fs::write(
        &file,
        indoc! {"
            Heading
            =======

            Paragraph.
        "},
    )
    .unwrap();

    let status = Command::new(mdlint_bin())
        .args(["format", "--check", file.to_str().unwrap()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();

    assert_eq!(
        status.code(),
        Some(1),
        "expected exit 1 when file needs formatting"
    );
}

#[test]
fn format_rewrites_file_in_place() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("doc.md");
    fs::write(
        &file,
        indoc! {"
            Heading
            =======

            * item
        "},
    )
    .unwrap();

    let status = Command::new(mdlint_bin())
        .args(["format", file.to_str().unwrap()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();

    assert!(status.success());
    let result = fs::read_to_string(&file).unwrap();
    assert_eq!(
        result,
        indoc! {"
            # Heading

            - item
        "}
    );
}

#[test]
fn cli_exclusions_apply_to_walked_and_explicit_files() {
    for command in ["format", "check"] {
        for source in ["cli", "config"] {
            for pattern in [
                "**/generated/**",
                "docs/generated/**",
                "**/skip.md",
                "docs/generated",
                "docs/generated/skip.md",
                "absolute-directory",
                "absolute-file",
            ] {
                let dir = TempDir::new().unwrap();
                let generated = dir.path().join("docs/generated");
                fs::create_dir_all(&generated).unwrap();
                let skipped = generated.join("skip.md");
                let kept = dir.path().join("docs/keep.md");
                fs::write(&skipped, "* item\n").unwrap();
                fs::write(&kept, "* item\n").unwrap();
                let exclude = match pattern {
                    "absolute-directory" => generated.to_str().unwrap(),
                    "absolute-file" => skipped.to_str().unwrap(),
                    _ => pattern,
                };

                let mut invocation = Command::new(mdlint_bin());
                invocation
                    .args([command, "docs", "docs/generated/skip.md"])
                    .current_dir(dir.path())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null());
                if command == "check" {
                    invocation.args(["--select", "MD004", "--fix"]);
                }
                if source == "cli" {
                    invocation.args(["--no-config", "--exclude", exclude]);
                } else {
                    let config = dir.path().join("mdlint.toml");
                    let value = toml::Value::String(exclude.to_owned());
                    fs::write(&config, format!("exclude = [{value}]\n")).unwrap();
                    invocation.args(["--config", config.to_str().unwrap()]);
                }
                let status = invocation.status().unwrap();
                assert_eq!(status.code(), Some(i32::from(command == "check")));
                assert_eq!(
                    fs::read_to_string(&skipped).unwrap(),
                    "* item\n",
                    "{command}: {source}: {pattern}"
                );
                assert_eq!(
                    fs::read_to_string(&kept).unwrap(),
                    "- item\n",
                    "{command}: {source}: {pattern}"
                );
            }
        }
    }
}

#[test]
fn invalid_exclusion_globs_are_reported() {
    let dir = TempDir::new().unwrap();
    let output = Command::new(mdlint_bin())
        .args(["--no-config", "format", "--exclude", "[unclosed"])
        .current_dir(dir.path())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Invalid glob pattern"), "{stderr}");
}

#[test]
fn runtime_errors_are_reported_on_stderr() {
    let dir = TempDir::new().unwrap();
    let config = dir.path().join("invalid.toml");
    fs::write(&config, "default_enabled = [").unwrap();

    let output = Command::new(mdlint_bin())
        .args(["--config", config.to_str().unwrap(), "format"])
        .current_dir(dir.path())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Configuration error"), "{stderr}");
}

// ── `mdlint check` CLI ───────────────────────────────────────────────────────

#[test]
fn check_without_fix_does_not_modify_file() {
    // `check` with `fix = false` must never write to disk.
    // We supply an explicit config because the default has `fix = true`.
    let dir = TempDir::new().unwrap();
    let config = dir.path().join("mdlint.toml");
    fs::write(
        &config,
        indoc! {"
            default_enabled = true
            fix = false
        "},
    )
    .unwrap();
    let file = dir.path().join("doc.md");
    let content = "# Heading\n\nTrailing spaces.   \n";
    fs::write(&file, content).unwrap();

    Command::new(mdlint_bin())
        .args([
            "check",
            "--config",
            config.to_str().unwrap(),
            file.to_str().unwrap(),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();

    let after = fs::read_to_string(&file).unwrap();
    assert_eq!(
        after, content,
        "check with fix=false must not modify the file"
    );
}

#[test]
fn check_reports_fix_failures_as_runtime_errors() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("doc.md");
    let original = "Text\n# Heading\nText\n";
    fs::write(&file, original).unwrap();
    let output = Command::new(mdlint_bin())
        .args(["--no-config", "check", "--select", "MD022", "--fix"])
        .arg(&file)
        .current_dir(dir.path())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("overlapping fix ranges"), "{stderr}");
    assert_eq!(fs::read_to_string(file).unwrap(), original);
}

#[test]
fn check_with_fix_corrects_violations_and_exits_1() {
    // `check --fix` applies inline fixes but still exits 1 because violations
    // were present (exit code reflects the pre-fix lint result).
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("doc.md");
    fs::write(&file, "# Heading\n\nTrailing spaces.   \n").unwrap();

    let status = Command::new(mdlint_bin())
        .args(["check", "--fix", file.to_str().unwrap()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();

    assert_eq!(
        status.code(),
        Some(1),
        "check --fix should exit 1 when violations were found"
    );
    let after = fs::read_to_string(&file).unwrap();
    assert_eq!(
        after,
        indoc! {"
            # Heading

            Trailing spaces.
        "},
        "trailing spaces should be removed by --fix"
    );
}
