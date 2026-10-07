use crate::error::Result;
use crate::glob::FileWalker;
use std::path::PathBuf;

pub fn find_files(
    paths: &[PathBuf],
    excludes: &[PathBuf],
    respect_ignore: bool,
) -> Result<Vec<PathBuf>> {
    let mut all_files = Vec::new();
    let mut add_to_file = |path: PathBuf| {
        if !all_files.contains(&path) && !is_excluded(&path, excludes) {
            all_files.push(path);
        }
    };

    for path in paths {
        if path.is_dir() {
            let walker = FileWalker::new(respect_ignore);
            walker
                .find_markdown_files(path)?
                .into_iter()
                .for_each(&mut add_to_file);
        } else if path.is_file() {
            add_to_file(path.clone());
        } else {
            eprintln!("Warning: Path not found: {}", path.display());
        }
    }

    Ok(all_files)
}

fn is_excluded(path: &PathBuf, excludes: &[PathBuf]) -> bool {
    excludes.iter().any(|exclude| {
        // Canonicalize the exclude path so relative paths (e.g. "FORMAT_SPEC.md")
        // match against the absolute paths returned by the file walker.
        if let Ok(canonical) = exclude.canonicalize() {
            path == &canonical || path.starts_with(&canonical)
        } else {
            path == exclude || path.starts_with(exclude)
        }
    })
}
