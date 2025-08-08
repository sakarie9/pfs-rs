//! Builder for creating PF8 archives.

use crate::entry::Pf8Entry;
use crate::error::{Error, Result};
use crate::writer::Pf8Writer;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// A builder for creating PF8 archives with a fluent API
pub struct Pf8Builder {
    /// Files to include in the archive
    files: Vec<(PathBuf, PathBuf)>, // (source_path, archive_path)
    /// Patterns for files that should not be encrypted
    unencrypted_patterns: Vec<String>,
    /// Base path for relative file paths
    base_path: Option<PathBuf>,
}

impl Pf8Builder {
    /// Creates a new builder for PF8 format
    pub fn new() -> Self {
        Self {
            files: Vec::new(),
            unencrypted_patterns: Vec::new(),
            base_path: None,
        }
    }

    /// Sets the base path for relative file paths
    pub fn base_path<P: AsRef<Path>>(&mut self, path: P) -> &mut Self {
        self.base_path = Some(path.as_ref().to_path_buf());
        self
    }

    /// Adds patterns for files that should not be encrypted
    pub fn unencrypted_extensions(&mut self, extensions: &[&str]) -> &mut Self {
        self.unencrypted_patterns
            .extend(extensions.iter().map(|&ext| ext.to_string()));
        self
    }

    /// Adds custom unencrypted patterns
    pub fn unencrypted_patterns(&mut self, patterns: &[&str]) -> &mut Self {
        self.unencrypted_patterns
            .extend(patterns.iter().map(|&pattern| pattern.to_string()));
        self
    }

    /// Adds a single file to the archive
    pub fn add_file<P: AsRef<Path>>(&mut self, file_path: P) -> Result<&mut Self> {
        let file_path = file_path.as_ref();

        if !file_path.exists() {
            return Err(Error::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("File not found: {}", file_path.display()),
            )));
        }

        if !file_path.is_file() {
            return Err(Error::InvalidFormat(format!(
                "Path is not a file: {}",
                file_path.display()
            )));
        }

        let archive_path = if let Some(base) = &self.base_path {
            file_path
                .strip_prefix(base)
                .map_err(|_| {
                    Error::InvalidFormat(format!(
                        "File path '{}' is not under base path '{}'",
                        file_path.display(),
                        base.display()
                    ))
                })?
                .to_path_buf()
        } else {
            file_path
                .file_name()
                .ok_or_else(|| Error::InvalidFormat("Invalid file name".to_string()))?
                .into()
        };

        self.files.push((file_path.to_path_buf(), archive_path));
        Ok(self)
    }

    /// Adds a single file with a custom archive path
    pub fn add_file_as<P: AsRef<Path>, Q: AsRef<Path>>(
        &mut self,
        file_path: P,
        archive_path: Q,
    ) -> Result<&mut Self> {
        let file_path = file_path.as_ref();

        if !file_path.exists() {
            return Err(Error::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("File not found: {}", file_path.display()),
            )));
        }

        if !file_path.is_file() {
            return Err(Error::InvalidFormat(format!(
                "Path is not a file: {}",
                file_path.display()
            )));
        }

        self.files
            .push((file_path.to_path_buf(), archive_path.as_ref().to_path_buf()));
        Ok(self)
    }

    /// Adds all files from a directory recursively
    pub fn add_dir<P: AsRef<Path>>(&mut self, dir_path: P) -> Result<&mut Self> {
        let dir_path = dir_path.as_ref();

        if !dir_path.exists() {
            return Err(Error::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Directory not found: {}", dir_path.display()),
            )));
        }

        if !dir_path.is_dir() {
            return Err(Error::InvalidFormat(format!(
                "Path is not a directory: {}",
                dir_path.display()
            )));
        }

        for entry in WalkDir::new(dir_path) {
            let entry = entry?;
            let file_path = entry.path();

            if file_path.is_file() {
                let relative_path = file_path.strip_prefix(dir_path).map_err(|_| {
                    Error::InvalidFormat("Failed to create relative path".to_string())
                })?;

                self.files
                    .push((file_path.to_path_buf(), relative_path.to_path_buf()));
            }
        }

        Ok(self)
    }

    /// Adds files from a directory with a custom archive prefix
    pub fn add_dir_as<P: AsRef<Path>, Q: AsRef<Path>>(
        &mut self,
        dir_path: P,
        archive_prefix: Q,
    ) -> Result<&mut Self> {
        let dir_path = dir_path.as_ref();
        let archive_prefix = archive_prefix.as_ref();

        if !dir_path.exists() {
            return Err(Error::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Directory not found: {}", dir_path.display()),
            )));
        }

        if !dir_path.is_dir() {
            return Err(Error::InvalidFormat(format!(
                "Path is not a directory: {}",
                dir_path.display()
            )));
        }

        for entry in WalkDir::new(dir_path) {
            let entry = entry?;
            let file_path = entry.path();

            if file_path.is_file() {
                let relative_path = file_path.strip_prefix(dir_path).map_err(|_| {
                    Error::InvalidFormat("Failed to create relative path".to_string())
                })?;

                let archive_path = archive_prefix.join(relative_path);
                self.files.push((file_path.to_path_buf(), archive_path));
            }
        }

        Ok(self)
    }

    /// Writes the archive to a file
    pub fn write_to_file<P: AsRef<Path>>(&self, output_path: P) -> Result<()> {
        let mut writer = Pf8Writer::create(output_path)?;
        self.write_to_writer(&mut writer)
    }

    /// Writes the archive using the provided writer
    pub fn write_to_writer(&self, writer: &mut Pf8Writer) -> Result<()> {
        if self.files.is_empty() {
            return Err(Error::InvalidFormat("No files to archive".to_string()));
        }

        // Build entries with metadata
        let mut entries = Vec::new();
        let mut total_data_size = 0u32;

        for (source_path, archive_path) in &self.files {
            let metadata = fs::metadata(source_path)?;
            let size = metadata.len();

            if size > u32::MAX as u64 {
                return Err(Error::InvalidFormat(format!(
                    "File too large: {} bytes (max: {} bytes)",
                    size,
                    u32::MAX
                )));
            }

            let size = size as u32;
            let patterns: Vec<&str> = self
                .unencrypted_patterns
                .iter()
                .map(|s| s.as_str())
                .collect();
            let entry = Pf8Entry::new(archive_path, total_data_size, size, &patterns);

            entries.push((entry, source_path.clone()));
            total_data_size += size;
        }

        // Write header and entries
        writer.write_header(&entries.iter().map(|(entry, _)| entry).collect::<Vec<_>>())?;

        // Write file data
        for (entry, source_path) in entries {
            let data = fs::read(source_path)?;
            writer.write_file_data(&entry, &data)?;
        }

        writer.finalize()?;
        Ok(())
    }

    /// Returns the number of files that will be included
    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    /// Returns true if no files have been added
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Clears all added files
    pub fn clear(&mut self) -> &mut Self {
        self.files.clear();
        self
    }

    /// Gets a list of all files that will be archived
    pub fn files(&self) -> impl Iterator<Item = (&Path, &Path)> {
        self.files
            .iter()
            .map(|(source, archive)| (source.as_path(), archive.as_path()))
    }
}

impl Default for Pf8Builder {
    fn default() -> Self {
        Self::new()
    }
}
