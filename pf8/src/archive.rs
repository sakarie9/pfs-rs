//! High-level archive operations for PF6/PF8 files.
//!
//! This module provides support for reading both PF6 and PF8 archive formats,
//! and creating PF8 archives. PF6 archives are read-only and do not use encryption,
//! while PF8 archives support both reading and writing with encryption capabilities.

use crate::builder::Pf8Builder;
use crate::callbacks::ArchiveHandler;
use crate::error::Result;
use crate::reader::Pf8Reader;
use std::ops::{Deref, DerefMut};
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

    /// Extracts a specific file to the given path using streaming I/O
    pub fn extract_file<P: AsRef<Path>, Q: AsRef<Path>>(
        &mut self,
        archive_path: P,
        output_path: Q,
    ) -> Result<()> {
        // Create parent directories if they don't exist
        if let Some(parent) = output_path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Use streaming extraction to avoid loading entire file into memory
        use std::fs::File;
        use std::io::Write;

        let mut output_file = File::create(output_path)?;
        self.reader.read_file_streaming(archive_path, |chunk| {
            output_file.write_all(chunk)?;
            Ok(())
        })?;

        Ok(())
    }

    /// Extracts a specific file with progress reporting
    pub fn extract_file_with_progress<P: AsRef<Path>, Q: AsRef<Path>, H: ArchiveHandler>(
        &mut self,
        archive_path: P,
        output_path: Q,
        handler: &mut H,
    ) -> Result<()> {
        self.reader
            .extract_file_with_progress(archive_path, output_path, handler)
    }

    /// Gets the underlying reader (for advanced use cases)
    pub fn reader(&self) -> &Pf8Reader {
        &self.reader
    }

    /// Gets the underlying reader mutably (for advanced use cases)
    pub fn reader_mut(&mut self) -> &mut Pf8Reader {
        &mut self.reader
    }
}

impl Deref for Pf8Archive {
    type Target = Pf8Reader;

    fn deref(&self) -> &Self::Target {
        &self.reader
    }
}

impl DerefMut for Pf8Archive {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.reader
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

/// Extracts a PF8 archive to the specified directory with progress reporting
pub fn extract_with_progress<P: AsRef<Path>, Q: AsRef<Path>, H: ArchiveHandler>(
    archive_path: P,
    output_dir: Q,
    handler: &mut H,
) -> Result<()> {
    let mut archive = Pf8Archive::open(archive_path)?;
    archive.extract_all_with_progress(output_dir, handler)
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
