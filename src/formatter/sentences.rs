use std::ops::Range;

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

/// Join a soft break that continues the same sentence. Do not split sentences.
/// Broken references still protect their labels; resolving them is unnecessary
/// for locating prose boundaries.
pub(super) fn format(text: &str, options: Options) -> String {
    let mut resolve = |_link: pulldown_cmark::BrokenLink<'_>| Some(("".into(), "".into()));
    let parser = Parser::new_with_broken_link_callback(text, options, Some(&mut resolve));
    let mut protected_depth = 0;
    let mut visible = String::new();
    let mut joins = Vec::new();
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
            Event::Text(_) | Event::Code(_) => visible.push_str(&text[range]),
            Event::SoftBreak if protected_depth == 0 => {
                if continues_sentence(&visible, &text[range.end..]) {
                    joins.push(soft_break_gap(text, range));
                }
                visible.clear();
            }
            Event::HardBreak => visible.clear(),
            _ => {}
        }
    }
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0;
    for gap in joins {
        out.push_str(&text[cursor..gap.start]);
        out.push(' ');
        cursor = gap.end;
    }
    out.push_str(&text[cursor..]);
    out
}

fn soft_break_gap(text: &str, range: Range<usize>) -> Range<usize> {
    let start = text[..range.start]
        .rfind(|c: char| !matches!(c, ' ' | '\t'))
        .map_or(range.start, |index| {
            text[..index].len() + text[index..].chars().next().map_or(0, char::len_utf8)
        });
    let mut end = range.end;
    while matches!(text.as_bytes().get(end), Some(b' ' | b'\t')) {
        end += 1;
    }
    start..end
}

fn continues_sentence(before: &str, after: &str) -> bool {
    let before = before.trim();
    let after = after.trim_start_matches(['\n', ' ', '\t']);
    if before.is_empty() || after.is_empty() || line_starts_new_block(after) {
        return false;
    }
    let after_sentence = after.trim_start_matches(['"', '\'', '“', '‘', '(']);
    if sentence_has_ended(before)
        && after_sentence
            .chars()
            .next()
            .is_some_and(char::is_uppercase)
    {
        return false;
    }
    starts_sentence(before) && wrapped_sentence_continues(before, after)
}

fn line_starts_new_block(after: &str) -> bool {
    let line = after.lines().next().unwrap_or("");
    line.starts_with(['#', '>', '|', '-', '+', '*'])
        || line.starts_with("```")
        || line.starts_with("~~~")
        || line.chars().next().is_some_and(|ch| ch.is_ascii_digit()) && line.contains(". ")
}

fn wrapped_sentence_continues(before: &str, after: &str) -> bool {
    let next = after
        .trim_start_matches(['"', '\'', '“', '‘', '('])
        .chars()
        .next();
    next.is_some_and(char::is_lowercase)
        && (sentence_is_open(before) || !sentence_has_ended(before))
}

fn sentence_is_open(before: &str) -> bool {
    let mut end = before.len();
    while before[..end].ends_with([')', ']', '"', '\'', '”', '’']) {
        let Some(mark) = before[..end].chars().next_back() else {
            break;
        };
        end -= mark.len_utf8();
    }
    before[..end]
        .chars()
        .next_back()
        .is_some_and(|ch| matches!(ch, ',' | ':' | ';' | '—' | '–' | '-'))
}

fn starts_sentence(text: &str) -> bool {
    text.trim_start_matches(['"', '\'', '“', '‘', '('])
        .chars()
        .next()
        .is_some_and(char::is_uppercase)
}

fn sentence_has_ended(before: &str) -> bool {
    let mut end = before.len();
    while before[..end].ends_with([')', ']', '"', '\'', '”', '’']) {
        let Some(mark) = before[..end].chars().next_back() else {
            break;
        };
        end -= mark.len_utf8();
    }
    let Some(punctuation) = before[..end].chars().next_back() else {
        return false;
    };
    matches!(punctuation, '.' | '!' | '?')
        && !before[..end].ends_with("...")
        && !(punctuation == '.' && ambiguous_period(&before[..end]))
}

fn ambiguous_period(prefix: &str) -> bool {
    let trimmed = prefix.trim_end_matches([')', ']', '"', '\'', '”', '’']);
    let Some(before_period) = trimmed.strip_suffix('.') else {
        return false;
    };
    let token = before_period
        .split_whitespace()
        .last()
        .unwrap_or("")
        .trim_matches(['(', ')', '[', ']', '"', '\'', '“', '”', '‘', '’']);
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
