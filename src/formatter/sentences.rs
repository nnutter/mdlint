use std::ops::Range;

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

/// Put clear sentence boundaries on their own lines and join wrapped lines that
/// continue the same sentence. Broken references still protect their labels;
/// resolving them is unnecessary for locating prose boundaries.
pub(super) fn format(text: &str, options: Options) -> String {
    let mut resolve = |_link: pulldown_cmark::BrokenLink<'_>| Some(("".into(), "".into()));
    let parser = Parser::new_with_broken_link_callback(text, options, Some(&mut resolve));
    let events: Vec<_> = parser.into_offset_iter().collect();
    let mut protected_depth = 0;
    let mut visible = String::new();
    let mut starts_with_inline_code = false;
    let mut edits = Vec::new();
    for (event_index, (event, range)) in events.iter().enumerate() {
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
            Event::Text(_) => {
                let mut current_sentence_start = None;
                if protected_depth == 0 {
                    for gap in sentence_gaps(&text[range.clone()]) {
                        edits.push((range.start + gap.start..range.start + gap.end, '\n'));
                        current_sentence_start = Some(gap.end);
                    }
                }
                if let Some(start) = current_sentence_start {
                    visible.clear();
                    visible.push_str(&text[range.start + start..range.end]);
                    starts_with_inline_code = false;
                } else {
                    visible.push_str(&text[range.clone()]);
                }
            }
            Event::Code(_) => {
                if visible
                    .trim_start_matches([' ', '\t', '"', '\'', '“', '‘', '('])
                    .is_empty()
                {
                    starts_with_inline_code = true;
                }
                visible.push_str(&text[range.clone()]);
            }
            Event::SoftBreak if protected_depth == 0 => {
                let next = first_sentence_char(&events[event_index + 1..]);
                if continues_sentence(&visible, &text[range.end..], next, starts_with_inline_code) {
                    edits.push((soft_break_gap(text, range.clone()), ' '));
                    visible.push(' ');
                } else {
                    visible.clear();
                    starts_with_inline_code = false;
                }
            }
            Event::HardBreak => {
                visible.clear();
                starts_with_inline_code = false;
            }
            _ => {}
        }
    }
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0;
    for (gap, replacement) in edits {
        out.push_str(&text[cursor..gap.start]);
        out.push(replacement);
        cursor = gap.end;
    }
    out.push_str(&text[cursor..]);
    out
}

fn first_sentence_char(events: &[(Event<'_>, Range<usize>)]) -> Option<char> {
    for (event, _) in events {
        let content = match event {
            Event::Text(text) | Event::Code(text) => text.as_ref(),
            Event::Start(Tag::Emphasis | Tag::Strong | Tag::Strikethrough | Tag::Link { .. })
            | Event::End(
                TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough | TagEnd::Link,
            ) => continue,
            _ => return None,
        };
        for ch in content.chars() {
            if !ch.is_whitespace() && !matches!(ch, '"' | '\'' | '“' | '‘' | '(') {
                return Some(ch);
            }
        }
    }
    None
}

fn sentence_gaps(text: &str) -> Vec<Range<usize>> {
    let mut gaps = Vec::new();
    for (index, punctuation) in text.char_indices() {
        if !matches!(punctuation, '.' | '!' | '?') {
            continue;
        }
        let mut end = index + punctuation.len_utf8();
        while let Some(c) = text[end..].chars().next() {
            if !matches!(c, ')' | ']' | '"' | '\'' | '”' | '’') {
                break;
            }
            end += c.len_utf8();
        }
        if !sentence_has_ended(&text[..end]) {
            continue;
        }
        let start = end;
        while matches!(text.as_bytes().get(end), Some(b' ' | b'\t')) {
            end += 1;
        }
        if start == end {
            continue;
        }
        let next = text[end..].trim_start_matches(['"', '\'', '“', '‘', '(']);
        if next.chars().next().is_some_and(char::is_uppercase) {
            gaps.push(start..end);
        }
    }
    gaps
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

fn continues_sentence(
    before: &str,
    after: &str,
    next: Option<char>,
    starts_with_inline_code: bool,
) -> bool {
    let before = before.trim();
    let after = after.trim_start_matches(['\n', ' ', '\t']);
    if before.is_empty() || after.is_empty() || next.is_none() || line_starts_new_block(after) {
        return false;
    }
    if sentence_has_ended(before) && next.is_some_and(char::is_uppercase) {
        return false;
    }
    (starts_with_inline_code || starts_sentence(before)) && wrapped_sentence_continues(before, next)
}

fn line_starts_new_block(after: &str) -> bool {
    let line = after.lines().next().unwrap_or("");
    let heading_marker_width = line.chars().take_while(|&ch| ch == '#').count();
    let is_heading = (1..=6).contains(&heading_marker_width)
        && line[heading_marker_width..].starts_with([' ', '\t']);
    let is_list_item = ["-", "+", "*"].iter().any(|marker| {
        line.strip_prefix(marker)
            .is_some_and(|rest| rest.starts_with([' ', '\t']))
    });
    let digits_end = line.bytes().take_while(u8::is_ascii_digit).count();
    let after_number = &line[digits_end..];
    let is_ordered_list = digits_end > 0
        && after_number
            .strip_prefix('.')
            .or_else(|| after_number.strip_prefix(')'))
            .is_some_and(|rest| rest.starts_with([' ', '\t']));
    let is_reference_definition = line.starts_with('[') && line.contains("]:");
    is_heading
        || line.starts_with(['>', '|'])
        || is_list_item
        || is_ordered_list
        || is_reference_definition
        || line.starts_with("```")
        || line.starts_with("~~~")
}

fn wrapped_sentence_continues(before: &str, next: Option<char>) -> bool {
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
