//! File entry representation and operations.

use crate::crypto;
use crate::error::{Error, Result};
use crate::format::{ArchiveFormat, RawEntry};
use crate::utils;
use std::path::{Path, PathBuf};

/// Represents a file entry in a PF8 archive
#[derive(Debug, Clone)]
pub struct Pf8Entry {
    /// Internal raw entry data
    raw: RawEntry,
    /// Cached normalized path
    path: PathBuf,
    /// Whether this entry should be encrypted
    encrypted: bool,
}

impl Pf8Entry {
    /// Creates a new entry from raw data
    pub fn from_raw(raw: RawEntry, unencrypted_patterns: &[&str]) -> Self {
        let path = utils::pf8_path_to_pathbuf(&raw.name.trim_end_matches('\0'));
        let encrypted = !utils::matches_any_pattern(&raw.name, unencrypted_patterns);

        Self {
            raw,
            path,
            encrypted,
        }
    }

    /// Creates a new entry from raw data with format awareness
    pub fn from_raw_with_format(
        raw: RawEntry,
        unencrypted_patterns: &[&str],
        format: ArchiveFormat,
    ) -> Self {
        let path = utils::pf8_path_to_pathbuf(&raw.name.trim_end_matches('\0'));
        // In PF6 format, no files are encrypted
        let encrypted = match format {
            ArchiveFormat::Pf6 => false,
            ArchiveFormat::Pf8 => !utils::matches_any_pattern(&raw.name, unencrypted_patterns),
        };

        Self {
            raw,
            path,
            encrypted,
        }
    }

    /// Creates a new entry for building archives
    pub fn new<P: AsRef<Path>>(
        path: P,
        offset: u32,
        size: u32,
        unencrypted_patterns: &[&str],
    ) -> Self {
        let path_ref = path.as_ref();
        let pf8_name = utils::pathbuf_to_pf8_path(path_ref);
        let encrypted = !utils::matches_any_pattern(&pf8_name, unencrypted_patterns);

        Self {
            raw: RawEntry {
                name: pf8_name,
                offset,
                size,
            },
            path: path_ref.to_path_buf(),
            encrypted,
        }
    }

    /// Gets the file path within the archive
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Gets the file name
    pub fn file_name(&self) -> Option<&str> {
        self.path.file_name().and_then(|os_str| os_str.to_str())
    }

    /// Gets the file size in bytes
    pub fn size(&self) -> u32 {
        self.raw.size
    }

    /// Gets the offset of the file data in the archive
    pub fn offset(&self) -> u32 {
        self.raw.offset
    }

    /// Returns whether this file is encrypted
    pub fn is_encrypted(&self) -> bool {
        self.encrypted
    }

    /// Gets the raw PF8 path string
    pub fn pf8_path(&self) -> &str {
        &self.raw.name
    }

    /// Reads the file data from the archive
    pub fn read(&self, archive_data: &[u8], encryption_key: Option<&[u8]>) -> Result<Vec<u8>> {
        let start = self.raw.offset as usize;
        let end = start + self.raw.size as usize;

        if end > archive_data.len() {
            return Err(Error::Corrupted(format!(
                "File data extends beyond archive bounds: {} > {}",
                end,
                archive_data.len()
            )));
        }

        let data = &archive_data[start..end];

        if self.encrypted {
            if let Some(key) = encryption_key {
                Ok(crypto::decrypt(data, key))
            } else {
                Err(Error::Crypto(
                    "File is encrypted but no key provided".to_string(),
                ))
            }
        } else {
            Ok(data.to_vec())
        }
    }

    /// Reads file data into the provided buffer
    pub fn read_into(
        &self,
        archive_data: &[u8],
        buffer: &mut [u8],
        encryption_key: Option<&[u8]>,
    ) -> Result<()> {
        if buffer.len() != self.raw.size as usize {
            return Err(Error::InvalidFormat(format!(
                "Buffer size mismatch: expected {}, got {}",
                self.raw.size,
                buffer.len()
            )));
        }

        let start = self.raw.offset as usize;
        let end = start + self.raw.size as usize;

        if end > archive_data.len() {
            return Err(Error::Corrupted(format!(
                "File data extends beyond archive bounds: {} > {}",
                end,
                archive_data.len()
            )));
        }

        let data = &archive_data[start..end];

        if self.encrypted {
            if let Some(key) = encryption_key {
                for (i, &byte) in data.iter().enumerate() {
                    buffer[i] = byte ^ key[i % key.len()];
                }
            } else {
                return Err(Error::Crypto(
                    "File is encrypted but no key provided".to_string(),
                ));
            }
        } else {
            buffer.copy_from_slice(data);
        }

        Ok(())
    }
}

impl PartialEq for Pf8Entry {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

impl Eq for Pf8Entry {}

impl std::hash::Hash for Pf8Entry {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.path.hash(state);
    }
}
