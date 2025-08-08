//! Utility functions for path handling and string operations.

use std::path::{Path, PathBuf};

/// Converts a PF8-style filename (backslash-separated) to a PathBuf
pub fn pf8_path_to_pathbuf(pf8_path: &str) -> PathBuf {
    pf8_path.split('\\').collect()
}

/// Converts a PathBuf to a PF8-style filename (backslash-separated)
pub fn pathbuf_to_pf8_path(path: &Path) -> String {
    path.iter()
        .map(|component| component.to_string_lossy())
        .collect::<Vec<_>>()
        .join("\\")
}

/// Checks if a file path matches any of the given patterns
pub fn matches_any_pattern(path: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|&pattern| {
        if pattern.starts_with('.') {
            // Extension pattern
            path.ends_with(pattern)
        } else {
            // Exact match or contains pattern
            path == pattern || path.contains(pattern)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pf8_path_conversion() {
        let pf8_path = "folder\\subfolder\\file.txt";
        let pathbuf = pf8_path_to_pathbuf(pf8_path);

        assert_eq!(pathbuf.to_string_lossy(), "folder/subfolder/file.txt");

        let converted_back = pathbuf_to_pf8_path(&pathbuf);
        assert_eq!(converted_back, pf8_path);
    }

    #[test]
    fn test_pattern_matching() {
        assert!(matches_any_pattern("file.txt", &[".txt"]));
        assert!(matches_any_pattern("file.TXT", &[".TXT"]));
        assert!(!matches_any_pattern("file.bin", &[".txt"]));

        assert!(matches_any_pattern("config.ini", &["config.ini"]));
        assert!(matches_any_pattern("path/config.ini", &["config.ini"]));
    }
}
