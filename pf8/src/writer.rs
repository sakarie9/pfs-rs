//! Writer for creating PF8 archives.

use crate::constants::BUFFER_SIZE;
use crate::crypto;
use crate::entry::Pf8Entry;
use crate::error::{Error, Result};
use crate::format;
use std::fs::{File, OpenOptions};
use std::io::{Seek, Write};
use std::path::Path;

/// Minimal information needed for encryption
#[derive(Debug, Clone)]
struct EncryptionInfo {
    is_encrypted: bool,
    size: u32,
}

/// A writer for creating PF8 archives
pub struct Pf8Writer {
    /// The output file
    output: File,
    /// Header buffer (only stores header data)
    header_data: Vec<u8>,
    /// Current state of the writer
    state: WriterState,
    /// Minimal encryption info for each entry
    encryption_info: Vec<EncryptionInfo>,
    /// Position where file data starts
    data_start_pos: u64,
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
        let output = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(output_path)?;

        Ok(Self {
            output,
            header_data: Vec::new(),
            state: WriterState::Created,
            encryption_info: Vec::new(),
            data_start_pos: 0,
        })
    }

    /// Writes the archive header with file entries
    pub fn write_header(&mut self, entries: &[&Pf8Entry]) -> Result<()> {
        if self.state != WriterState::Created {
            return Err(Error::InvalidFormat("Header already written".to_string()));
        }

        // Store minimal encryption info for later use during finalization
        self.encryption_info = entries
            .iter()
            .map(|entry| EncryptionInfo {
                is_encrypted: entry.is_encrypted(),
                size: entry.size(),
            })
            .collect();

        // Calculate sizes
        let index_count = entries.len() as u32;
        let mut fileentry_size = 0usize;

        for entry in entries {
            fileentry_size += entry.pf8_path().len() + 16; // name + padding + offset + size
        }

        let index_size = (4 + fileentry_size + 4 + (index_count as usize + 1) * 8 + 4) as u32;

        // Build header in memory (only header data, not file content)
        self.header_data.clear();
        self.header_data.extend_from_slice(format::PF8_MAGIC);
        self.header_data
            .extend_from_slice(&index_size.to_le_bytes());
        self.header_data
            .extend_from_slice(&index_count.to_le_bytes());

        // Write file entries
        let mut file_offset = index_size + format::offsets::INDEX_DATA_START as u32;
        let mut filesize_offsets = Vec::new();

        for entry in entries {
            let name_bytes = entry.pf8_path().as_bytes();
            let name_length = name_bytes.len() as u32;

            // name_length
            self.header_data
                .extend_from_slice(&name_length.to_le_bytes());
            // name
            self.header_data.extend_from_slice(name_bytes);
            // reserved
            self.header_data
                .extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // padding
            // offset
            self.header_data
                .extend_from_slice(&file_offset.to_le_bytes());
            // size
            self.header_data
                .extend_from_slice(&entry.size().to_le_bytes());

            // Track the offset of the size field for later use
            // offset from faddr 0xf
            filesize_offsets.push(
                (self.header_data.len() - 4 - format::offsets::FILESIZE_OFFSETS_START) as u64,
            );
            file_offset += entry.size();
        }

        // Write filesize count and offsets
        self.header_data
            .extend_from_slice(&(index_count + 1).to_le_bytes());

        let filesize_count_offset =
            (self.header_data.len() - 4 - format::offsets::INDEX_DATA_START) as u32;

        for offset in filesize_offsets {
            self.header_data.extend_from_slice(&offset.to_le_bytes());
        }

        // End marker
        self.header_data.extend_from_slice(&[0x00; 8]);

        // Write filesize_count_offset
        self.header_data
            .extend_from_slice(&filesize_count_offset.to_le_bytes());

        // Write header to file immediately
        self.output.write_all(&self.header_data)?;
        self.data_start_pos = self.output.stream_position()?;

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

        // Write data directly to file instead of buffering
        self.output.write_all(data)?;
        self.state = WriterState::WritingData;

        Ok(())
    }

    /// Finalizes the archive by applying encryption
    pub fn finalize(&mut self) -> Result<()> {
        if self.state == WriterState::Finalized {
            return Ok(());
        }

        if self.state == WriterState::Created {
            return Err(Error::InvalidFormat("No data written".to_string()));
        }

        // For in-place encryption, we need to read back the file data
        // This is still more memory efficient than keeping everything in memory

        // Generate encryption key from header
        let index_size = format::get_index_size(&self.header_data)?;
        let encryption_key = crypto::generate_key(&self.header_data, index_size);

        // Get current file position (end of file)
        let file_end = self.output.stream_position()?;

        // Process each file that needs encryption
        let mut data_offset = self.data_start_pos;

        for enc_info in &self.encryption_info {
            if enc_info.is_encrypted {
                // Read, encrypt, and write back in chunks
                let file_size = enc_info.size as u64;
                let mut processed = 0u64;

                while processed < file_size {
                    let chunk_len = std::cmp::min(BUFFER_SIZE, (file_size - processed) as usize);

                    // Read chunk
                    self.output
                        .seek(std::io::SeekFrom::Start(data_offset + processed))?;
                    let mut buffer = vec![0u8; chunk_len];
                    use std::io::Read;
                    self.output.read_exact(&mut buffer)?;

                    // Encrypt chunk with correct offset within this file
                    crypto::encrypt(&mut buffer, &encryption_key, processed as usize);

                    // Write back encrypted chunk
                    self.output
                        .seek(std::io::SeekFrom::Start(data_offset + processed))?;
                    self.output.write_all(&buffer)?;

                    processed += chunk_len as u64;
                }
            }

            data_offset += enc_info.size as u64;
        }

        // Restore file position to end
        self.output.seek(std::io::SeekFrom::Start(file_end))?;
        self.output.flush()?;

        self.state = WriterState::Finalized;
        Ok(())
    }

    /// Gets the current size of the archive
    pub fn size(&mut self) -> usize {
        // Return current file position
        self.output.stream_position().unwrap_or(0) as usize
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
