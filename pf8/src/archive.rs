//! High-level archive operations for PF6/PF8 files.
//!
//! This module provides support for reading both PF6 and PF8 archive formats,
//! and creating PF8 archives. PF6 archives are read-only and do not use encryption,
//! while PF8 archives support both reading and writing with encryption capabilities.

use crate::builder::Pf8Builder;
use crate::entry::Pf8Entry;
use crate::error::Result;
use crate::format::ArchiveFormat;
use crate::reader::Pf8Reader;
use std::path::Path;

/// High-level interface for working with PF6/PF8 archives
pub struct Pf8Archive {
    reader: Pf8Reader,
}

impl Pf8Archive {
    /// Opens an existing PF6/PF8 archive
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let reader = Pf8Reader::open(path)?;
        Ok(Self { reader })
    }

    /// Opens a PF6/PF8 archive with custom unencrypted patterns
    pub fn open_with_patterns<P: AsRef<Path>>(
        path: P,
        unencrypted_patterns: &[&str],
    ) -> Result<Self> {
        let reader = Pf8Reader::open_with_unencrypted_patterns(path, unencrypted_patterns)?;
        Ok(Self { reader })
    }

    /// Creates a new archive builder (PF8 format with encryption)
    pub fn builder() -> Pf8Builder {
        Pf8Builder::new()
    }

    /// Gets the archive format (PF6 or PF8)
    pub fn format(&self) -> ArchiveFormat {
        self.reader.format()
    }

    /// Returns true if the archive uses encryption (PF8 only)
    pub fn is_encrypted(&self) -> bool {
        self.reader.is_encrypted()
    }

    /// Returns an iterator over all file entries
    pub fn entries(&self) -> Result<impl Iterator<Item = &Pf8Entry>> {
        Ok(self.reader.entries())
    }

    /// Gets the number of files in the archive
    pub fn len(&self) -> usize {
        self.reader.len()
    }

    /// Returns true if the archive is empty
    pub fn is_empty(&self) -> bool {
        self.reader.is_empty()
    }

    /// Gets a file entry by path
    pub fn get_entry<P: AsRef<Path>>(&self, path: P) -> Result<Option<&Pf8Entry>> {
        Ok(self.reader.get_entry(path))
    }

    /// Checks if a file exists in the archive
    pub fn contains<P: AsRef<Path>>(&self, path: P) -> bool {
        self.reader.contains(path)
    }

    /// Reads a file's data by path
    pub fn read_file<P: AsRef<Path>>(&mut self, path: P) -> Result<Vec<u8>> {
        self.reader.read_file(path)
    }

    /// Reads a file's data with streaming to minimize memory allocation
    pub fn read_file_streaming<P: AsRef<Path>, F>(&mut self, path: P, callback: F) -> Result<()>
    where
        F: FnMut(&[u8]) -> Result<()>,
    {
        self.reader.read_file_streaming(path, callback)
    }

    /// Reads a file's data with streaming and custom buffer size
    pub fn read_file_streaming_with_buffer_size<P: AsRef<Path>, F>(
        &mut self,
        path: P,
        buffer_size: usize,
        callback: F,
    ) -> Result<()>
    where
        F: FnMut(&[u8]) -> Result<()>,
    {
        self.reader
            .read_file_streaming_with_buffer_size(path, buffer_size, callback)
    }

    /// Extracts all files to the specified directory
    pub fn extract_all<P: AsRef<Path>>(&mut self, output_dir: P) -> Result<()> {
        self.reader.extract_all(output_dir)
    }

    /// Extracts all files to the specified directory with specified buffer size for memory optimization
    pub fn extract_all_with_buffer_size<P: AsRef<Path>>(
        &mut self,
        output_dir: P,
        buffer_size: usize,
    ) -> Result<()> {
        self.reader
            .extract_all_with_buffer_size(output_dir, buffer_size)
    }

    /// Extracts a specific file to the given path
    pub fn extract_file<P: AsRef<Path>, Q: AsRef<Path>>(
        &mut self,
        archive_path: P,
        output_path: Q,
    ) -> Result<()> {
        let data = self.read_file(archive_path)?;

        // Create parent directories if they don't exist
        if let Some(parent) = output_path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(output_path, data)?;
        Ok(())
    }

    /// Gets the underlying reader (for advanced use cases)
    pub fn reader(&self) -> &Pf8Reader {
        &self.reader
    }
}

// Convenience functions for one-off operations

/// Extracts a PF8 archive to the specified directory
pub fn extract<P: AsRef<Path>, Q: AsRef<Path>>(archive_path: P, output_dir: Q) -> Result<()> {
    let mut archive = Pf8Archive::open(archive_path)?;
    archive.extract_all(output_dir)
}

/// Extracts a PF8 archive with custom unencrypted patterns
pub fn extract_with_patterns<P: AsRef<Path>, Q: AsRef<Path>>(
    archive_path: P,
    output_dir: Q,
    unencrypted_patterns: &[&str],
) -> Result<()> {
    let mut archive = Pf8Archive::open_with_patterns(archive_path, unencrypted_patterns)?;
    archive.extract_all(output_dir)
}

/// Creates a PF8 archive from a directory
pub fn create_from_dir<P: AsRef<Path>, Q: AsRef<Path>>(input_dir: P, output_path: Q) -> Result<()> {
    let mut builder = Pf8Builder::new();
    builder.add_dir(input_dir)?;
    builder.write_to_file(output_path)
}

/// Creates a PF8 archive from a directory with custom unencrypted patterns
pub fn create_from_dir_with_patterns<P: AsRef<Path>, Q: AsRef<Path>>(
    input_dir: P,
    output_path: Q,
    unencrypted_patterns: &[&str],
) -> Result<()> {
    let mut builder = Pf8Builder::new();
    builder.unencrypted_patterns(unencrypted_patterns);
    builder.add_dir(input_dir)?;
    builder.write_to_file(output_path)
}
