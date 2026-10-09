use crate::markdown::MarkdownParser;
use crate::types::{Fix, Violation};
use pulldown_cmark::{Event, LinkType, Tag, TagEnd};
use std::ops::Range;

pub(super) fn reference_identity_ranges(parser: &MarkdownParser) -> Vec<Range<usize>> {
    parser
        .parse_with_offsets()
        .filter_map(|(event, range)| {
            matches!(
                event,
                Event::Start(
                    Tag::Link {
                        link_type: LinkType::Shortcut | LinkType::Collapsed,
                        ..
                    } | Tag::Image {
                        link_type: LinkType::Shortcut | LinkType::Collapsed,
                        ..
                    }
                )
            )
            .then_some(range)
        })
        .collect()
}

pub(super) fn canonical_violations(
    parser: &MarkdownParser,
    strong: bool,
    rule: &str,
) -> Vec<Violation> {
    let reference_ranges = reference_identity_ranges(parser);
    let mut spans = Vec::new();
    let mut previous_end = false;
    let mut previous_marker = "";
    let mut violations = Vec::new();
    for (event, range) in parser.parse_with_offsets() {
        if reference_ranges
            .iter()
            .any(|label| label.contains(&range.start))
        {
            previous_end = false;
            continue;
        }
        let is_end = matches!(event, Event::End(TagEnd::Emphasis | TagEnd::Strong));
        match event {
            Event::Start(Tag::Emphasis | Tag::Strong) => {
                let is_strong = matches!(event, Event::Start(Tag::Strong));
                let alternate = previous_end && previous_marker.ends_with('*');
                let marker = crate::formatter::emphasis_delimiter(is_strong, alternate);
                spans.push((is_strong, marker, range));
            }
            Event::End(TagEnd::Emphasis | TagEnd::Strong) => {
                let (is_strong, marker, span) = spans.pop().expect("emphasis start precedes end");
                previous_marker = marker;
                if is_strong == strong {
                    for offset in [span.start, span.end - marker.len()] {
                        let actual = &parser.content()[offset..offset + marker.len()];
                        if actual != marker {
                            let (line, column) = parser.offset_to_position(offset);
                            let name = if strong { "Strong" } else { "Emphasis" };
                            violations.push(Violation {
                                line,
                                column: Some(column),
                                rule: rule.to_owned(),
                                message: format!(
                                    "{name} style should be '{marker}', found '{actual}'"
                                ),
                                fix: Some(Fix {
                                    line_start: line,
                                    line_end: line,
                                    column_start: Some(column),
                                    column_end: Some(column + marker.len() - 1),
                                    replacement: marker.to_owned(),
                                    description: format!("Replace {} marker", name.to_lowercase()),
                                }),
                            });
                        }
                    }
                }
            }
            _ => {}
        }
        previous_end = is_end;
    }
    violations
}
