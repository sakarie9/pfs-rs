//! Writer for creating PF8 archives.

use crate::constants::BUFFER_SIZE;
use crate::crypto;
use crate::entry::Pf8Entry;
use crate::error::{Error, Result};
use crate::format;
use std::fs::{File, OpenOptions};
use std::io::{Seek, Write};
use std::path::Path;

/// A writer for creating PF8 archives
pub struct Pf8Writer {
    /// The output file
    output: File,
    /// Header buffer (only stores header data)
    header_data: Vec<u8>,
    /// Current state of the writer
    state: WriterState,
    /// Position where file data starts
    data_start_pos: u64,
    /// Cached encryption key (computed once after header is written)
    encryption_key: Option<Vec<u8>>,
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
            data_start_pos: 0,
            encryption_key: None,
        })
    }

    /// Writes the archive header with file entries
    pub fn write_header(&mut self, entries: &[&Pf8Entry]) -> Result<()> {
        if self.state != WriterState::Created {
            return Err(Error::InvalidFormat("Header already written".to_string()));
        }

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

        // Generate and cache encryption key once
        let index_size = format::get_index_size(&self.header_data)?;
        self.encryption_key = Some(crypto::generate_key(&self.header_data, index_size));

        self.state = WriterState::HeaderWritten;
        Ok(())
    }

    /// Writes data for a file entry
    /// This method writes the file data directly to the output without buffering.
    /// It is suitable for small files or when low latency is required.
    /// But for larger files, it will cause very high memory usage as much of the file
    /// will be held in memory at once.
    /// Use write_file_data instead of this.
    pub fn write_file_data_direct(&mut self, entry: &Pf8Entry, data: &[u8]) -> Result<()> {
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

    /// Writes file data from a reader using streaming to minimize memory usage
    ///
    /// This method reads the source file in chunks (default 4MB) and writes them directly
    /// to the output, avoiding loading the entire file into memory. This is especially
    /// beneficial for large files that would otherwise cause high memory usage.
    ///
    /// If encryption is needed, it will be applied on-the-fly during the streaming process.
    pub fn write_file_data<P: AsRef<std::path::Path>>(
        &mut self,
        entry: &Pf8Entry,
        source_path: P,
    ) -> Result<()> {
        if self.state == WriterState::Created {
            return Err(Error::InvalidFormat(
                "Header must be written first".to_string(),
            ));
        }

        if self.state == WriterState::Finalized {
            return Err(Error::InvalidFormat("Writer is finalized".to_string()));
        }

        use std::io::Read;
        let mut source_file = std::fs::File::open(source_path)?;
        let expected_size = entry.size() as u64;
        let use_encryption = entry.is_encrypted();
        let mut total_written = 0u64;

        // For small files, read entirely to minimize overhead
        if expected_size <= BUFFER_SIZE as u64 {
            let mut data = vec![0u8; expected_size as usize];
            source_file.read_exact(&mut data)?;

            // Apply encryption if needed
            if use_encryption
                && self.encryption_key.is_some()
                && let Some(ref key) = self.encryption_key
            {
                crypto::encrypt(&mut data, key, 0);
            }

            // Write all at once
            self.output.write_all(&data)?;
            total_written = expected_size;
        } else {
            // For large files, use streaming with optimized buffer reuse
            let mut buffer = vec![0u8; BUFFER_SIZE];

            while total_written < expected_size {
                let remaining = expected_size - total_written;
                let chunk_size = std::cmp::min(BUFFER_SIZE as u64, remaining) as usize;

                // Read chunk from source file
                source_file.read_exact(&mut buffer[..chunk_size])?;

                // Apply encryption if needed, using cached key
                if use_encryption
                    && self.encryption_key.is_some()
                    && let Some(ref key) = self.encryption_key
                {
                    crypto::encrypt(&mut buffer[..chunk_size], key, total_written as usize);
                }

                // Write chunk to output (already encrypted if needed)
                self.output.write_all(&buffer[..chunk_size])?;

                total_written += chunk_size as u64;
            }
        }

        if total_written != expected_size {
            return Err(Error::InvalidFormat(format!(
                "Data size mismatch: expected {}, wrote {}",
                expected_size, total_written
            )));
        }

        self.state = WriterState::WritingData;
        Ok(())
    }

    /// Finalizes the archive
    ///
    /// Since encryption is now handled during the streaming write process,
    /// this method mainly ensures the writer is in a finalized state.
    pub fn finalize(&mut self) -> Result<()> {
        if self.state == WriterState::Finalized {
            return Ok(());
        }

        if self.state == WriterState::Created {
            return Err(Error::InvalidFormat("No data written".to_string()));
        }

        // Ensure all data is written to disk
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
