//! PF6/PF8 format constants and low-level parsing functions.

//    PF6/PF8 structure
//    |magic 'pf6' or 'pf8'
//    |index_size 4 //start from index_count (faddr 0x7)
//    |index_count 4
//    |file_entrys[]
//      |name_length 4
//      |name //string with '\0'
//      |00 00 00 00
//      |offset 4
//      |size 4
//    |filesize_count 4
//    |filesize_offsets[] 8 //offset from faddr 0xf, last is 00 00 00 00 00 00 00 00
//    |filesize_count_offset 4 //offset from faddr 0x7

use crate::error::{Error, Result};

/// PF6 magic number
pub const PF6_MAGIC: &[u8] = b"pf6";

/// PF8 magic number
pub const PF8_MAGIC: &[u8] = b"pf8";

/// Archive format type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveFormat {
    Pf6,
    Pf8,
}

/// PF8 format header offsets
pub mod offsets {
    pub const MAGIC: usize = 0x00;
    pub const INDEX_SIZE: usize = 0x03;
    pub const INDEX_COUNT: usize = 0x07;
    pub const ENTRIES_START: usize = 0x0B;
    pub const INDEX_DATA_START: usize = 0x07;
}

/// Raw file entry as stored in PF8 format
#[derive(Debug, Clone)]
pub struct RawEntry {
    pub name: String,
    pub offset: u32,
    pub size: u32,
}

/// Validates that the data starts with PF6 or PF8 magic number
pub fn validate_magic(data: &[u8]) -> Result<ArchiveFormat> {
    if data.len() < 3 {
        return Err(Error::InvalidFormat("Data too short".to_string()));
    }

    let magic = &data[offsets::MAGIC..offsets::MAGIC + 3];

    if magic == PF6_MAGIC {
        Ok(ArchiveFormat::Pf6)
    } else if magic == PF8_MAGIC {
        Ok(ArchiveFormat::Pf8)
    } else {
        Err(Error::InvalidFormat("Not a PF6 or PF8 file".to_string()))
    }
}

/// Reads a u32 from the given offset in little-endian format
pub fn read_u32_le(data: &[u8], offset: usize) -> Result<u32> {
    if offset + 4 > data.len() {
        return Err(Error::InvalidFormat(
            "Not enough data to read u32".to_string(),
        ));
    }

    Ok(u32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ]))
}

/// Parses the PF6/PF8 header and returns file entries along with format information
pub fn parse_entries(data: &[u8]) -> Result<(Vec<RawEntry>, ArchiveFormat)> {
    let format = validate_magic(data)?;

    if data.len() < 11 {
        return Err(Error::InvalidFormat(
            "Data too short to parse header".to_string(),
        ));
    }

    let index_size = read_u32_le(data, offsets::INDEX_SIZE)?;
    let index_count = read_u32_le(data, offsets::INDEX_COUNT)?;

    let mut file_entries = Vec::new();
    let mut cursor = offsets::ENTRIES_START;
    let index_end_pos = (offsets::INDEX_DATA_START + index_size as usize).min(data.len());

    while cursor < index_end_pos && file_entries.len() < index_count as usize {
        if cursor + 4 > data.len() {
            break;
        }

        let name_length = read_u32_le(data, cursor)?;
        cursor += 4;

        if cursor + name_length as usize + 12 > data.len() {
            break;
        }

        let name_bytes = &data[cursor..cursor + name_length as usize];
        let name = String::from_utf8(name_bytes.to_vec())?;
        cursor += name_length as usize + 4; // Skip name and 4 zero bytes

        let offset = read_u32_le(data, cursor)?;
        let size = read_u32_le(data, cursor + 4)?;
        cursor += 8;

        file_entries.push(RawEntry { name, offset, size });
    }

    if file_entries.len() != index_count as usize {
        return Err(Error::Corrupted(format!(
            "Index count mismatch. Expected {}, found {}",
            index_count,
            file_entries.len()
        )));
    }

    Ok((file_entries, format))
}

/// Gets the index size from PF6/PF8 header
pub fn get_index_size(data: &[u8]) -> Result<u32> {
    validate_magic(data)?;
    read_u32_le(data, offsets::INDEX_SIZE)
}
