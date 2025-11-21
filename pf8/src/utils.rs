//! Utility functions for path handling and string operations.

use std::path::{Path, PathBuf};

use crate::constants::UNENCRYPTED_FILTER;

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
pub fn matches_any_pattern(path: &str) -> bool {
    UNENCRYPTED_FILTER.to_vec().iter().any(|&pattern| {
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
}
