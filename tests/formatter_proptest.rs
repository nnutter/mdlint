use mdlint::formatter;
use proptest::prelude::*;
use pulldown_cmark::{CodeBlockKind, Event, LinkType, Options, Parser, Tag, TagEnd};

fn markdown_documents() -> impl Strategy<Value = String> {
    let word = "[a-zé😀]{1,12}";
    let block = prop_oneof![
        word.prop_map(|text| format!("{text}\n=======")),
        word.prop_map(|text| format!("## {text} ##")),
        word.prop_map(|text| format!("{text} _italic_ and __bold__ and \\*literal\\*.")),
        word.prop_map(|text| format!("_{text}_*second*")),
        word.prop_map(|text| format!("[{text}](https://example.com \"title\") ![alt](image.png)")),
        word.prop_map(|text| format!("[{text}][label]\n\n[label]: https://example.com \"title\"")),
        word.prop_map(|text| format!("* {text}\n  + nested\n* second")),
        word.prop_map(|text| format!("* [ ] {text}\n* [x] second")),
        word.prop_map(|text| format!("~~{text}~~")),
        word.prop_map(|text| format!("{text}[^note]\n\n[^note]: body")),
        word.prop_map(|text| format!("    {text}")),
        word.prop_map(|text| format!("3. {text}\n8. second")),
        word.prop_map(|text| format!("> {text}\n>\n> _second_")),
        word.prop_map(|text| format!("| {text} | name |\n| :--- | ---: |\n| value | __bold__ |")),
        word.prop_map(|text| format!("<div>\n{text}\n</div>")),
        word.prop_map(|text| format!("{text}  \nhard break and `` a ` b ``")),
        prop::sample::select(vec!["let x = 1;", "```", "````", "\tcode"])
            .prop_map(|code| format!("~~~rust\n{code}\n~~~")),
    ];
    prop::collection::vec(block, 1..8).prop_map(|blocks| format!("{}\n", blocks.join("\n\n")))
}

fn semantic_events(input: &str) -> Vec<Event<'static>> {
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_HEADING_ATTRIBUTES;
    let mut events = Vec::new();
    let mut parsed = Parser::new_ext(input, options).peekable();
    while let Some(event) = parsed.next() {
        // Empty comment blocks separate lists without adding rendered content.
        if matches!(event, Event::Start(Tag::HtmlBlock))
            && matches!(parsed.peek(), Some(Event::Html(html)) if html.trim() == "<!---->")
        {
            parsed.next();
            assert_eq!(parsed.next(), Some(Event::End(TagEnd::HtmlBlock)));
            continue;
        }
        let mut event = event.into_static();
        match &mut event {
            // Fence syntax is not semantic. Ordered starts are canonicalized to 1.
            Event::Start(Tag::CodeBlock(kind @ CodeBlockKind::Indented)) => {
                *kind = CodeBlockKind::Fenced("".into());
            }
            Event::Start(Tag::List(Some(start))) => *start = 1,
            // Resolved references and inline links have the same rendered target.
            Event::Start(Tag::Link { link_type, id, .. } | Tag::Image { link_type, id, .. }) => {
                *link_type = LinkType::Inline;
                *id = "".into();
            }
            _ => {}
        }
        // Escaping can change Text-event boundaries without changing the text.
        if let Event::Text(text) = &event
            && let Some(Event::Text(previous)) = events.last_mut()
        {
            *previous = format!("{previous}{text}").into();
            continue;
        }
        events.push(event);
    }
    events
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10_000))]

    #[test]
    fn formatter_preserves_document_semantics(input in markdown_documents()) {
        let formatted = formatter::format(&input);
        prop_assert_eq!(
            semantic_events(&input),
            semantic_events(&formatted),
            "semantic change on input: {:?}, output: {:?}", input, formatted
        );
        prop_assert_eq!(formatter::format(&formatted), formatted);
    }

    /// The formatter must never panic on any input string.
    #[test]
    fn formatter_never_panics(s in ".*") {
        let _ = formatter::format(&s);
    }

    /// Formatting is idempotent: format(format(x)) == format(x).
    #[test]
    fn formatter_is_idempotent(s in ".*") {
        let once = formatter::format(&s);
        let twice = formatter::format(&once);
        prop_assert_eq!(once, twice, "formatter not idempotent on input: {:?}", s);
    }

    /// The formatted output must end with exactly one newline (or be empty).
    #[test]
    fn formatter_trailing_newline(s in ".+") {
        let out = formatter::format(&s);
        if !out.is_empty() {
            prop_assert!(out.ends_with('\n'), "output should end with newline: {out:?}");
            prop_assert!(
                !out.ends_with("\n\n"),
                "output should not have double trailing newline: {out:?}"
            );
        }
    }

    /// Prose lines must not have trailing whitespace; code contents are verbatim.
    #[test]
    fn formatter_no_trailing_whitespace(s in ".*") {
        let out = formatter::format(&s);
        let parser = mdlint::markdown::MarkdownParser::new(&out);
        for (index, line) in out.lines().enumerate() {
            if parser.get_code_block_line_numbers().contains(&(index + 1)) {
                continue;
            }
            prop_assert!(
                !line.ends_with(' ') && !line.ends_with('\t'),
                "line has trailing whitespace: {line:?}"
            );
        }
    }
}
