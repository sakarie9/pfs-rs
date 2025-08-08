//! Writer for creating PF8 archives.

use crate::crypto;
use crate::entry::Pf8Entry;
use crate::error::{Error, Result};
use crate::format;
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// A writer for creating PF8 archives
pub struct Pf8Writer {
    /// The output file
    output: File,
    /// Archive data buffer
    data: Vec<u8>,
    /// Current state of the writer
    state: WriterState,
    /// Stored entries with their encryption information
    entries: Vec<Pf8Entry>,
}

#[derive(Debug, PartialEq)]
enum WriterState {
    Created,
    HeaderWritten,
    WritingData,
    Finalized,
}

impl Pf8Writer {
    /// Creates a new writer for the given output file
    pub fn create<P: AsRef<Path>>(output_path: P) -> Result<Self> {
        let output = File::create(output_path)?;

        Ok(Self {
            output,
            data: Vec::new(),
            state: WriterState::Created,
            entries: Vec::new(),
        })
    }

    /// Writes the archive header with file entries
    pub fn write_header(&mut self, entries: &[&Pf8Entry]) -> Result<()> {
        if self.state != WriterState::Created {
            return Err(Error::InvalidFormat("Header already written".to_string()));
        }

        // Store entries for later use during finalization
        self.entries = entries.iter().cloned().cloned().collect();

        // Calculate sizes
        let index_count = entries.len() as u32;
        let mut fileentry_size = 0usize;

        for entry in entries {
            fileentry_size += entry.pf8_path().len() + 16; // name + padding + offset + size
        }

        let index_size = (4 + fileentry_size + 4 + (index_count as usize + 1) * 8 + 4) as u32;

        // Write magic, index_size, and index_count
        self.data.extend_from_slice(format::PF8_MAGIC);
        self.data.extend_from_slice(&index_size.to_le_bytes());
        self.data.extend_from_slice(&index_count.to_le_bytes());

        // Write file entries
        let mut file_offset = index_size + format::offsets::INDEX_DATA_START as u32;
        let mut filesize_offsets = Vec::new();

        for entry in entries {
            let name_bytes = entry.pf8_path().as_bytes();
            let name_length = name_bytes.len() as u32;

            self.data.extend_from_slice(&name_length.to_le_bytes());
            self.data.extend_from_slice(name_bytes);
            self.data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // padding
            self.data.extend_from_slice(&file_offset.to_le_bytes());
            self.data.extend_from_slice(&entry.size().to_le_bytes());

            // Track the offset of the size field for later use
            filesize_offsets.push((self.data.len() - 4 - format::offsets::ENTRIES_START) as u64);
            file_offset += entry.size();
        }

        // Write filesize count and offsets
        self.data
            .extend_from_slice(&(index_count + 1).to_le_bytes());

        let filesize_count_offset =
            (self.data.len() - 4 - format::offsets::INDEX_DATA_START) as u32;

        for offset in filesize_offsets {
            self.data.extend_from_slice(&offset.to_le_bytes());
        }

        // End marker
        self.data.extend_from_slice(&[0x00; 8]);

        // Write filesize_count_offset
        self.data
            .extend_from_slice(&filesize_count_offset.to_le_bytes());

        self.state = WriterState::HeaderWritten;
        Ok(())
    }

    /// Writes data for a file entry
    pub fn write_file_data(&mut self, entry: &Pf8Entry, data: &[u8]) -> Result<()> {
        if self.state == WriterState::Created {
            return Err(Error::InvalidFormat(
                "Header must be written first".to_string(),
            ));
        }

        if self.state == WriterState::Finalized {
            return Err(Error::InvalidFormat("Writer is finalized".to_string()));
        }

        if data.len() != entry.size() as usize {
            return Err(Error::InvalidFormat(format!(
                "Data size mismatch: expected {}, got {}",
                entry.size(),
                data.len()
            )));
        }

        // Store the data for later encryption
        self.data.extend_from_slice(data);
        self.state = WriterState::WritingData;

        Ok(())
    }

    /// Finalizes the archive by applying encryption and writing to file
    pub fn finalize(&mut self) -> Result<()> {
        if self.state == WriterState::Finalized {
            return Ok(());
        }

        if self.state == WriterState::Created {
            return Err(Error::InvalidFormat("No data written".to_string()));
        }

        // Generate encryption key from header
        let index_size = format::get_index_size(&self.data)?;
        let encryption_key = crypto::generate_key(&self.data, index_size);

        // Apply encryption to file data using stored entry information
        let mut data_offset = format::offsets::INDEX_DATA_START + index_size as usize;

        for entry in &self.entries {
            if entry.is_encrypted() && data_offset + entry.size() as usize <= self.data.len() {
                crypto::encrypt(
                    &mut self.data[data_offset..data_offset + entry.size() as usize],
                    &encryption_key,
                );
            }

            data_offset += entry.size() as usize;
        }

        // Write all data to file
        self.output.write_all(&self.data)?;
        self.output.flush()?;

        self.state = WriterState::Finalized;
        Ok(())
    }

    /// Gets the current size of the archive
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// Checks if the writer is finalized
    pub fn is_finalized(&self) -> bool {
        self.state == WriterState::Finalized
    }
}

impl Drop for Pf8Writer {
    fn drop(&mut self) {
        if self.state != WriterState::Finalized {
            // Try to finalize on drop, but ignore errors
            let _ = self.finalize();
        }
    }
}
