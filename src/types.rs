use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct FileResult {
    pub path: PathBuf,
    pub violations: Vec<Violation>,
    /// Source lines (1-indexed by line number) used for snippet display.
    /// May be empty if source is not available.
    pub source_lines: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Violation {
    pub line: usize,
    /// 1-indexed UTF-8 byte column within the source line.
    pub column: Option<usize>,
    pub rule: String,
    pub message: String,
    pub fix: Option<Fix>,
}

#[derive(Debug, Clone)]
pub struct Fix {
    /// 1-indexed, inclusive source line range.
    pub line_start: usize,
    pub line_end: usize,
    /// 1-indexed, inclusive UTF-8 byte columns for a single-line edit.
    /// The start byte and the byte after the end must be character boundaries.
    /// With no columns, the fix replaces whole lines.
    pub column_start: Option<usize>,
    pub column_end: Option<usize>,
    pub replacement: String,
    pub description: String,
}
