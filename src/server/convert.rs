use crate::error::Result;
use crate::fix::Fixer;
use crate::types::{Fix, Violation};
use lsp_types::{Diagnostic, DiagnosticSeverity, NumberOrString, Position, Range, TextEdit, Uri};
use std::path::PathBuf;

/// Convert a UTF-8 byte offset to a UTF-16 code unit offset within a line.
#[allow(clippy::cast_possible_truncation)] // UTF-16 code units per char is 1 or 2; sum fits u32
fn byte_offset_to_utf16(line_text: &str, byte_offset: usize) -> u32 {
    line_text
        .char_indices()
        .take_while(|(offset, _)| *offset < byte_offset)
        .map(|(_, c)| c.len_utf16() as u32)
        .sum()
}

/// Convert a `Violation` to an LSP `Diagnostic`.
///
/// mdlint uses 1-indexed lines and columns; LSP uses 0-indexed UTF-16 positions.
#[allow(clippy::cast_possible_truncation)] // LSP positions are u32; line counts in real files fit
pub fn violation_to_diagnostic(v: &Violation, content: &str) -> Diagnostic {
    let lines: Vec<&str> = content.lines().collect();
    let lsp_line = v.line.saturating_sub(1) as u32;
    let lsp_char = match v.column {
        None => 0,
        Some(col) => {
            let byte_offset = col.saturating_sub(1);
            lines
                .get(v.line.saturating_sub(1))
                .map_or(0, |line| byte_offset_to_utf16(line, byte_offset))
        }
    };
    let position = Position {
        line: lsp_line,
        character: lsp_char,
    };
    Diagnostic {
        range: Range {
            start: position,
            end: position,
        },
        severity: Some(DiagnosticSeverity::WARNING),
        code: Some(NumberOrString::String(v.rule.clone())),
        source: Some("mdlint".to_owned()),
        message: v.message.clone(),
        ..Default::default()
    }
}

/// Convert a `Fix` to an LSP `TextEdit`.
///
/// Whole-line fixes use the shared fixer and replace the document so newline
/// insertion and deletion have exactly the same semantics as CLI fixes.
#[allow(clippy::cast_possible_truncation)] // LSP positions are u32; line counts in real files fit
pub fn fix_to_text_edit(fix: &Fix, content: &str) -> Result<TextEdit> {
    let corrected = Fixer::new().apply_fixes_to_content(content, std::slice::from_ref(fix))?;
    let lines: Vec<&str> = content.lines().collect();

    if fix.line_start != fix.line_end || fix.column_start.is_none() || fix.column_end.is_none() {
        Ok(whole_doc_edit(content, &corrected))
    } else {
        let start_line = fix.line_start.saturating_sub(1);
        let end_line = fix.line_end.saturating_sub(1);
        let start_byte = fix.column_start.map_or(0, |c| c.saturating_sub(1));
        let end_byte = fix.column_end.unwrap_or(0);

        let start_utf16 = lines
            .get(start_line)
            .map_or(0, |l| byte_offset_to_utf16(l, start_byte));
        let end_utf16 = lines
            .get(end_line)
            .map_or(0, |l| byte_offset_to_utf16(l, end_byte));

        Ok(TextEdit {
            range: Range {
                start: Position {
                    line: start_line as u32,
                    character: start_utf16,
                },
                end: Position {
                    line: end_line as u32,
                    character: end_utf16,
                },
            },
            new_text: fix.replacement.clone(),
        })
    }
}

/// Build a `TextEdit` that replaces the entire document with `formatted`.
///
/// The end position is the actual EOF, including when the last line has no newline.
#[allow(clippy::cast_possible_truncation)] // LSP positions are u32; line counts in real files fit
pub fn whole_doc_edit(content: &str, formatted: &str) -> TextEdit {
    let line_count = content.bytes().filter(|&byte| byte == b'\n').count() as u32;
    let last_line = content.rsplit('\n').next().unwrap_or_default();
    TextEdit {
        range: Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: line_count,
                character: byte_offset_to_utf16(last_line, last_line.len()),
            },
        },
        new_text: formatted.to_owned(),
    }
}

/// Convert a `file://` URI to a `PathBuf`. Returns `None` for non-file schemes.
pub fn uri_to_path(uri: &Uri) -> Option<PathBuf> {
    url::Url::parse(uri.as_str()).ok()?.to_file_path().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Violation;
    use indoc::indoc;
    use std::str::FromStr;

    fn make_violation(line: usize, column: Option<usize>) -> Violation {
        Violation {
            line,
            column,
            rule: "MD001".to_owned(),
            message: "test".to_owned(),
            fix: None,
        }
    }

    #[test]
    fn test_coord_no_column() {
        let v = make_violation(3, None);
        let diag = violation_to_diagnostic(
            &v,
            indoc! {"
                line1
                line2
                line3
            "},
        );
        assert_eq!(diag.range.start.line, 2);
        assert_eq!(diag.range.start.character, 0);
    }

    #[test]
    fn test_coord_ascii() {
        // 1-indexed col 5 → LSP character 4
        let v = make_violation(1, Some(5));
        let diag = violation_to_diagnostic(&v, "hello world\n");
        assert_eq!(diag.range.start.line, 0);
        assert_eq!(diag.range.start.character, 4);
    }

    #[test]
    fn test_coord_utf16() {
        // Byte column 6 points to 'c', after the four-byte emoji and 'b'.
        let content = "\u{1F600}bc\n";
        let v = make_violation(1, Some(6));
        let diag = violation_to_diagnostic(&v, content);
        assert_eq!(diag.range.start.character, 3);
    }

    #[test]
    fn fix_edits_replace_the_complete_range() {
        for (content, column_start, column_end, replacement, expected) in [
            ("é😀   \n", Some(7), Some(9), "", "é😀\n"),
            ("é __😀__\n", Some(4), Some(5), "**", "é **😀__\n"),
            ("é\nnext\n", None, None, "new", "new\nnext\n"),
            ("é\r\nnext\r\n", None, None, "new", "new\r\nnext\r\n"),
            ("é\nnext\n", None, None, "", "next\n"),
            ("é", None, None, "é\n", "é\n"),
        ] {
            let fix = Fix {
                line_start: 1,
                line_end: 1,
                column_start,
                column_end,
                replacement: replacement.to_owned(),
                description: "test".to_owned(),
            };
            let edit = fix_to_text_edit(&fix, content).unwrap();
            let actual = apply_edit(content, &edit);
            assert_eq!(actual, expected);
        }
    }

    fn byte_offset(content: &str, position: Position) -> usize {
        let mut offset = 0;
        for (index, line) in content.split('\n').enumerate() {
            if index == position.line as usize {
                let mut utf16 = 0;
                for (byte, ch) in line.char_indices() {
                    if utf16 == position.character {
                        return offset + byte;
                    }
                    utf16 += u32::try_from(ch.len_utf16()).unwrap();
                }
                assert_eq!(utf16, position.character);
                return offset + line.len();
            }
            offset += line.len() + 1;
        }
        panic!("position outside document: {position:?}");
    }

    fn apply_edit(content: &str, edit: &TextEdit) -> String {
        let start = byte_offset(content, edit.range.start);
        let end = byte_offset(content, edit.range.end);
        let mut result = content.to_owned();
        result.replace_range(start..end, &edit.new_text);
        result
    }

    #[test]
    fn whole_document_ranges_end_at_the_actual_eof() {
        for content in ["", "é😀", "é😀\n", "é😀\r\n", "a\nb"] {
            let edit = whole_doc_edit(content, "replacement\n");
            assert_eq!(apply_edit(content, &edit), "replacement\n");
        }
    }

    #[test]
    fn test_uri_file_scheme() {
        let uri = Uri::from_str("file:///tmp/foo.md").unwrap();
        let path = uri_to_path(&uri).unwrap();
        assert_eq!(path, PathBuf::from("/tmp/foo.md"));
    }

    #[test]
    fn test_uri_non_file() {
        let uri = Uri::from_str("untitled:foo.md").unwrap();
        assert!(uri_to_path(&uri).is_none());
    }
}
