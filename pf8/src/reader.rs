//! High-level reader for PF6/PF8 archives.

use crate::constants::{BUFFER_SIZE, UNENCRYPTED_FILTER};
use crate::crypto;
use crate::entry::Pf8Entry;
use crate::error::{Error, Result};
use crate::format::{self, ArchiveFormat};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

/// Optimized reader for PF6/PF8 archives with minimal memory usage
///
/// This reader minimizes memory usage by:
/// - Not memory-mapping the entire file
/// - Reading file data on-demand from disk
/// - Supporting streaming operations with configurable buffers
pub struct Pf8Reader {
    /// File handle for reading archive data
    file: File,
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
    /// Opens a PF6/PF8 archive for reading with minimal memory usage
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        Self::open_with_unencrypted_patterns(path, &UNENCRYPTED_FILTER)
    }

    /// Creates a reader with custom unencrypted patterns
    pub fn open_with_unencrypted_patterns<P: AsRef<Path>>(
        path: P,
        unencrypted_patterns: &[&str],
    ) -> Result<Self> {
        let mut file = File::open(path)?;

        // Read only the header and index data into memory
        let header_size = 11; // minimum header size
        let mut header_buffer = vec![0u8; header_size];
        file.read_exact(&mut header_buffer)?;

        let _format = format::validate_magic(&header_buffer)?;
        let index_size = format::read_u32_le(&header_buffer, format::offsets::INDEX_SIZE)?;

        // Read the entire index into memory
        let total_index_size = format::offsets::INDEX_DATA_START + index_size as usize;
        let mut index_buffer = vec![0u8; total_index_size];
        file.seek(SeekFrom::Start(0))?;
        file.read_exact(&mut index_buffer)?;

        let (raw_entries, format) = format::parse_entries(&index_buffer)?;

        // Generate encryption key only for PF8 format
        let encryption_key = match format {
            ArchiveFormat::Pf8 => Some(crypto::generate_key(&index_buffer, index_size)),
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
            file,
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
    pub fn read_file<P: AsRef<Path>>(&mut self, path: P) -> Result<Vec<u8>> {
        // Get entry info and copy values to avoid borrow conflicts
        let (offset, size, is_encrypted) = {
            let entry = self
                .get_entry(path)
                .ok_or_else(|| Error::FileNotFound("File not found".to_string()))?;
            (entry.offset(), entry.size(), entry.is_encrypted())
        };

        self.read_entry_data_by_params(offset, size, is_encrypted)
    }

    /// Reads a file's data with streaming to minimize memory allocation
    pub fn read_file_streaming<P: AsRef<Path>, F>(&mut self, path: P, mut callback: F) -> Result<()>
    where
        F: FnMut(&[u8]) -> Result<()>,
    {
        // Get entry info and copy values to avoid borrow conflicts
        let (file_size, start_offset, is_encrypted) = {
            let entry = self
                .get_entry(path)
                .ok_or_else(|| Error::FileNotFound("File not found".to_string()))?;
            (
                entry.size() as usize,
                entry.offset() as u64,
                entry.is_encrypted(),
            )
        };

        self.file.seek(SeekFrom::Start(start_offset))?;

        if file_size <= BUFFER_SIZE {
            // Small file: read directly
            let mut data = vec![0u8; file_size];
            self.file.read_exact(&mut data)?;

            if is_encrypted {
                if let Some(key) = self.encryption_key.as_deref() {
                    for (i, byte) in data.iter_mut().enumerate() {
                        *byte ^= key[i % key.len()];
                    }
                } else {
                    return Err(Error::Crypto(
                        "File is encrypted but no key provided".to_string(),
                    ));
                }
            }

            callback(&data)?;
        } else {
            // Large file: stream in chunks
            let mut buffer = vec![0u8; BUFFER_SIZE];
            let mut bytes_read = 0;

            while bytes_read < file_size {
                let chunk_size = (file_size - bytes_read).min(BUFFER_SIZE);
                self.file.read_exact(&mut buffer[..chunk_size])?;

                if is_encrypted {
                    if let Some(key) = self.encryption_key.as_deref() {
                        // Decrypt chunk in-place
                        for (i, byte) in buffer[..chunk_size].iter_mut().enumerate() {
                            *byte ^= key[(bytes_read + i) % key.len()];
                        }
                    } else {
                        return Err(Error::Crypto(
                            "File is encrypted but no key provided".to_string(),
                        ));
                    }
                }

                callback(&buffer[..chunk_size])?;
                bytes_read += chunk_size;
            }
        }

        Ok(())
    }

    /// Extracts all files to the specified directory with memory optimization
    pub fn extract_all<P: AsRef<Path>>(&mut self, output_dir: P) -> Result<()> {
        let output_dir = output_dir.as_ref();
        let mut buffer = vec![0u8; BUFFER_SIZE];

        for entry in &self.entries.clone() {
            let file_path = output_dir.join(entry.path());

            // Create parent directories if they don't exist
            if let Some(parent) = file_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            self.extract_entry_streaming(entry, &file_path, &mut buffer)?;
        }

        Ok(())
    }

    /// Extracts a single entry using streaming to minimize memory usage
    fn extract_entry_streaming<P: AsRef<Path>>(
        &mut self,
        entry: &Pf8Entry,
        output_path: P,
        buffer: &mut [u8],
    ) -> Result<()> {
        use std::io::Write;

        let mut output_file = File::create(output_path)?;

        // Copy entry info to avoid borrow conflicts
        let (file_size, start_offset, is_encrypted) = {
            (
                entry.size() as usize,
                entry.offset() as u64,
                entry.is_encrypted(),
            )
        };

        self.file.seek(SeekFrom::Start(start_offset))?;

        if file_size <= buffer.len() {
            // Small file: read directly into buffer
            let mut temp_buffer = vec![0u8; file_size];
            self.file.read_exact(&mut temp_buffer)?;

            if is_encrypted {
                if let Some(key) = self.encryption_key.as_deref() {
                    for (i, byte) in temp_buffer.iter_mut().enumerate() {
                        *byte ^= key[i % key.len()];
                    }
                } else {
                    return Err(Error::Crypto(
                        "File is encrypted but no key provided".to_string(),
                    ));
                }
            }

            output_file.write_all(&temp_buffer)?;
        } else {
            // Large file: stream in chunks
            let buffer_size = buffer.len();
            let mut bytes_written = 0;

            while bytes_written < file_size {
                let chunk_size = (file_size - bytes_written).min(buffer_size);
                self.file.read_exact(&mut buffer[..chunk_size])?;

                if is_encrypted {
                    if let Some(key) = self.encryption_key.as_deref() {
                        for (i, byte) in buffer[..chunk_size].iter_mut().enumerate() {
                            *byte ^= key[(bytes_written + i) % key.len()];
                        }
                    } else {
                        return Err(Error::Crypto(
                            "File is encrypted but no key provided".to_string(),
                        ));
                    }
                }

                output_file.write_all(&buffer[..chunk_size])?;
                bytes_written += chunk_size;
            }
        }

        Ok(())
    }

    /// Reads entry data from file by parameters
    fn read_entry_data_by_params(
        &mut self,
        offset: u32,
        size: u32,
        is_encrypted: bool,
    ) -> Result<Vec<u8>> {
        let start_offset = offset as u64;
        let size = size as usize;

        self.file.seek(SeekFrom::Start(start_offset))?;
        let mut data = vec![0u8; size];
        self.file.read_exact(&mut data)?;

        if is_encrypted {
            if let Some(key) = self.encryption_key.as_deref() {
                for (i, byte) in data.iter_mut().enumerate() {
                    *byte ^= key[i % key.len()];
                }
            } else {
                return Err(Error::Crypto(
                    "File is encrypted but no key provided".to_string(),
                ));
            }
        }

        Ok(data)
    }
}
