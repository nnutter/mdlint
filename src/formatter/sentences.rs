use std::ops::Range;

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

/// Only split ordinary text. Broken references still protect their labels;
/// resolving them is unnecessary for locating prose boundaries.
pub(super) fn format(text: &str, options: Options) -> String {
    let mut resolve = |_link: pulldown_cmark::BrokenLink<'_>| Some(("".into(), "".into()));
    let parser = Parser::new_with_broken_link_callback(text, options, Some(&mut resolve));
    let mut protected_depth = 0;
    let mut breaks = Vec::new();
    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Start(
                Tag::Link { .. }
                | Tag::Image { .. }
                | Tag::Emphasis
                | Tag::Strong
                | Tag::Strikethrough,
            ) => protected_depth += 1,
            Event::End(
                TagEnd::Link
                | TagEnd::Image
                | TagEnd::Emphasis
                | TagEnd::Strong
                | TagEnd::Strikethrough,
            ) => protected_depth -= 1,
            Event::InlineHtml(_) | Event::Html(_) => return text.to_owned(),
            Event::Text(_) if protected_depth == 0 => {
                for gap in sentence_gaps(&text[range.clone()]) {
                    breaks.push((range.start + gap.start)..(range.start + gap.end));
                }
            }
            _ => {}
        }
    }
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0;
    for gap in breaks {
        out.push_str(&text[cursor..gap.start]);
        out.push('\n');
        cursor = gap.end;
    }
    out.push_str(&text[cursor..]);
    out
}

fn sentence_gaps(text: &str) -> Vec<Range<usize>> {
    let mut gaps = Vec::new();
    for (index, punctuation) in text.char_indices() {
        if !matches!(punctuation, '.' | '!' | '?') {
            continue;
        }
        if punctuation == '.' && ambiguous_period(&text[..index]) {
            continue;
        }
        let mut end = index + 1;
        while let Some(c) = text[end..].chars().next() {
            if !matches!(c, ')' | ']' | '\"' | '\'' | '”' | '’') {
                break;
            }
            end += c.len_utf8();
        }
        let start = end;
        while matches!(text.as_bytes().get(end), Some(b' ' | b'\t')) {
            end += 1;
        }
        if start == end {
            continue;
        }
        let next = text[end..].trim_start_matches(['\"', '\'', '“', '‘', '(']);
        if next.chars().next().is_some_and(char::is_uppercase) {
            gaps.push(start..end);
        }
    }
    gaps
}

fn ambiguous_period(prefix: &str) -> bool {
    let token = prefix
        .split_whitespace()
        .last()
        .unwrap_or("")
        .trim_matches(['(', ')', '[', ']', '\"', '\'', '“', '”', '‘', '’']);
    // Internal dots cover initials, decimals, ellipses, domains, and filenames.
    // URL-like tokens and escaped punctuation are not safe sentence boundaries.
    if token.contains(['.', '/', '@', ':', '\\'])
        || (token.chars().count() == 1 && token.chars().all(char::is_alphabetic))
        || (!token.is_empty() && token.chars().all(char::is_numeric))
    {
        return true;
    }
    matches!(
        token.to_ascii_lowercase().as_str(),
        "dr" | "mr"
            | "mrs"
            | "ms"
            | "prof"
            | "sr"
            | "jr"
            | "st"
            | "vs"
            | "etc"
            | "cf"
            | "fig"
            | "no"
            | "inc"
            | "ltd"
            | "dept"
            | "approx"
            | "al"
            | "jan"
            | "feb"
            | "mar"
            | "apr"
            | "jun"
            | "jul"
            | "aug"
            | "sep"
            | "sept"
            | "oct"
            | "nov"
            | "dec"
    )
}
