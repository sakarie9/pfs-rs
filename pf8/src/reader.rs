//! High-level reader for PF6/PF8 archives.

use crate::crypto;
use crate::entry::Pf8Entry;
use crate::error::{Error, Result};
use crate::format::{self, ArchiveFormat};
use memmap2::Mmap;
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

const UNENCRYPTED_FILTER: [&str; 2] = ["mp4", "flv"];

/// A reader for PF6/PF8 archives that provides streaming access to files
pub struct Pf8Reader {
    /// Memory-mapped archive data
    data: Mmap,
    /// List of file entries
    entries: Vec<Pf8Entry>,
    /// Lookup map for fast entry access by path
    entry_map: HashMap<String, usize>,
    /// Encryption key for the archive (None for PF6)
    encryption_key: Option<Vec<u8>>,
    /// Archive format
    format: ArchiveFormat,
}

impl Pf8Reader {
    /// Opens a PF6/PF8 archive for reading
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path)?;
        let data = unsafe { Mmap::map(&file)? };
        Self::from_data(data, &UNENCRYPTED_FILTER)
    }

    /// Creates a reader with custom unencrypted patterns
    pub fn open_with_unencrypted_patterns<P: AsRef<Path>>(
        path: P,
        unencrypted_patterns: &[&str],
    ) -> Result<Self> {
        let file = File::open(path)?;
        let data = unsafe { Mmap::map(&file)? };
        Self::from_data(data, unencrypted_patterns)
    }

    /// Creates a reader with custom unencrypted patterns
    pub fn from_data(data: Mmap, unencrypted_patterns: &[&str]) -> Result<Self> {
        let (raw_entries, format) = format::parse_entries(&data)?;

        // Generate encryption key only for PF8 format
        let encryption_key = match format {
            ArchiveFormat::Pf8 => {
                let index_size = format::get_index_size(&data)?;
                Some(crypto::generate_key(&data, index_size))
            }
            ArchiveFormat::Pf6 => None,
        };

        let mut entries = Vec::with_capacity(raw_entries.len());
        let mut entry_map = HashMap::new();

        for (index, raw_entry) in raw_entries.into_iter().enumerate() {
            let entry = Pf8Entry::from_raw_with_format(raw_entry, unencrypted_patterns, format);
            let path_string = entry.path().to_string_lossy().to_string();
            entry_map.insert(path_string, index);
            entries.push(entry);
        }

        Ok(Self {
            data,
            entries,
            entry_map,
            encryption_key,
            format,
        })
    }

    /// Returns an iterator over all file entries
    pub fn entries(&self) -> impl Iterator<Item = &Pf8Entry> {
        self.entries.iter()
    }

    /// Gets the number of files in the archive
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns true if the archive is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Gets the archive format (PF6 or PF8)
    pub fn format(&self) -> ArchiveFormat {
        self.format
    }

    /// Returns true if the archive uses encryption (PF8 only)
    pub fn is_encrypted(&self) -> bool {
        self.encryption_key.is_some()
    }

    /// Gets a file entry by path
    pub fn get_entry<P: AsRef<Path>>(&self, path: P) -> Option<&Pf8Entry> {
        let path_string = path.as_ref().to_string_lossy().to_string();
        self.entry_map
            .get(&path_string)
            .map(|&index| &self.entries[index])
    }

    /// Checks if a file exists in the archive
    pub fn contains<P: AsRef<Path>>(&self, path: P) -> bool {
        self.get_entry(path).is_some()
    }

    /// Reads a file's data by path
    pub fn read_file<P: AsRef<Path>>(&self, path: P) -> Result<Vec<u8>> {
        let entry = self
            .get_entry(path)
            .ok_or_else(|| Error::FileNotFound("File not found".to_string()))?;

        entry.read(&self.data, self.encryption_key.as_deref())
    }

    /// Reads a file's data into the provided buffer
    pub fn read_file_into<P: AsRef<Path>>(&self, path: P, buffer: &mut [u8]) -> Result<()> {
        let entry = self
            .get_entry(path)
            .ok_or_else(|| Error::FileNotFound("File not found".to_string()))?;

        entry.read_into(&self.data, buffer, self.encryption_key.as_deref())
    }

    /// Extracts all files to the specified directory
    pub fn extract_all<P: AsRef<Path>>(&self, output_dir: P) -> Result<()> {
        let output_dir = output_dir.as_ref();

        for entry in &self.entries {
            let file_path = output_dir.join(entry.path());

            // Create parent directories if they don't exist
            if let Some(parent) = file_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let data = entry.read(&self.data, self.encryption_key.as_deref())?;
            std::fs::write(file_path, data)?;
        }

        Ok(())
    }

    /// Extracts a specific file to the given path
    pub fn extract_file<P: AsRef<Path>, Q: AsRef<Path>>(
        &self,
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
}
