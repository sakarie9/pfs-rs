//! Display functionality for PF8 archives (requires 'display' feature).

use crate::archive::Pf8Archive;
use crate::entry::Pf8Entry;
use crate::error::Result;
use std::fmt;
use std::path::Path;

#[cfg(feature = "display")]
use human_bytes::human_bytes;
#[cfg(feature = "display")]
use tabled::settings::object::Columns;
#[cfg(feature = "display")]
use tabled::settings::{Alignment, Style};
#[cfg(feature = "display")]
use tabled::{Table, Tabled};

/// Represents a file entry for display purposes
#[cfg(feature = "display")]
#[derive(Tabled)]
pub struct DisplayEntry {
    #[tabled(rename = "File")]
    pub name: String,
    #[tabled(rename = "Size", display = "Self::format_size")]
    pub size: u32,
}

#[cfg(feature = "display")]
impl DisplayEntry {
    fn format_size(size: &u32) -> String {
        human_bytes(*size as f64)
    }

    pub fn from_entry(entry: &Pf8Entry) -> Self {
        Self {
            name: entry.path().to_string_lossy().to_string(),
            size: entry.size(),
        }
    }
}

/// Represents a list of files in the PF8 archive for display
#[cfg(feature = "display")]
pub struct FileList {
    entries: Vec<DisplayEntry>,
}

#[cfg(feature = "display")]
impl FileList {
    pub fn new(entries: Vec<DisplayEntry>) -> Self {
        Self { entries }
    }

    pub fn from_archive(archive: &Pf8Archive) -> Result<Self> {
        let entries = archive.entries()?.map(DisplayEntry::from_entry).collect();
        Ok(Self { entries })
    }
}

#[cfg(feature = "display")]
impl fmt::Display for FileList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.entries.is_empty() {
            return write!(f, "Archive is empty");
        }

        let mut table = Table::new(&self.entries);
        table.with(Style::markdown());
        table.modify(Columns::last(), Alignment::right()); // Align size column

        let count = self.entries.len();
        let total_size: u64 = self.entries.iter().map(|e| e.size as u64).sum();

        let footer = format!(
            "Total: {} files, Total size: {}",
            count,
            human_bytes(total_size as f64)
        );

        write!(f, "{table}\n\n{footer}")
    }
}

/// Lists the contents of a PF8 archive in a formatted table
#[cfg(feature = "display")]
pub fn list_archive<P: AsRef<Path>>(archive_path: P) -> Result<()> {
    let archive = Pf8Archive::open(&archive_path)?;
    let file_list = FileList::from_archive(&archive)?;

    println!("{}", archive_path.as_ref().display());
    println!();
    println!("{file_list}");

    Ok(())
}

/// Lists the contents of a PF8 archive with custom unencrypted patterns
#[cfg(feature = "display")]
pub fn list_archive_with_patterns<P: AsRef<Path>>(
    archive_path: P,
    unencrypted_patterns: &[&str],
) -> Result<()> {
    let archive = Pf8Archive::open_with_patterns(&archive_path, unencrypted_patterns)?;
    let file_list = FileList::from_archive(&archive)?;

    println!("{}", archive_path.as_ref().display());
    println!();
    println!("{file_list}");

    Ok(())
}

// Fallback implementations when display feature is not enabled
#[cfg(not(feature = "display"))]
pub fn list_archive<P: AsRef<Path>>(_archive_path: P) -> Result<()> {
    Err(crate::error::Error::InvalidFormat(
        "Display functionality requires the 'display' feature".to_string(),
    ))
}

#[cfg(not(feature = "display"))]
pub fn list_archive_with_patterns<P: AsRef<Path>>(
    _archive_path: P,
    _unencrypted_patterns: &[&str],
) -> Result<()> {
    Err(crate::error::Error::InvalidFormat(
        "Display functionality requires the 'display' feature".to_string(),
    ))
}
