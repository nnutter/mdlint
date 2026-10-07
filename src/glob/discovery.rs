use crate::error::Result;
use crate::glob::{FileWalker, GlobMatcher};
use std::env;
use std::path::PathBuf;

pub fn find_files(
    paths: &[PathBuf],
    excludes: &[PathBuf],
    respect_ignore: bool,
) -> Result<Vec<PathBuf>> {
    let root = env::current_dir()?.canonicalize()?;
    let mut literal_excludes = Vec::new();
    let mut patterns = Vec::new();
    for exclude in excludes {
        if let Ok(canonical) = exclude.canonicalize() {
            literal_excludes.push(canonical);
        } else if let Some(pattern) = exclude.to_str()
            && pattern.contains(['*', '?', '[', '{'])
        {
            patterns.push(format!("#{pattern}"));
        } else {
            literal_excludes.push(root.join(exclude));
        }
    }
    let matcher = GlobMatcher::new(&patterns)?;
    let mut all_files = Vec::new();
    let mut add_to_file = |path: PathBuf| {
        let relative = path.strip_prefix(&root).unwrap_or(&path);
        if !all_files.contains(&path)
            && !literal_excludes
                .iter()
                .any(|exclude| path.starts_with(exclude))
            && matcher.matches(relative)
            && matcher.matches(&path)
        {
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
            add_to_file(path.canonicalize()?);
        } else {
            eprintln!("Warning: Path not found: {}", path.display());
        }
    }

    Ok(all_files)
}
