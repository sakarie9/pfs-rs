//! Cryptographic operations for PF8 files.

use crate::format;
use sha1::{Digest, Sha1};

/// Generates encryption key from PF8 archive header data
pub fn generate_key(data: &[u8], index_size: u32) -> Vec<u8> {
    let start = format::offsets::INDEX_DATA_START;
    let end = start + index_size as usize;

    if end > data.len() {
        // Fallback to available data if index_size is larger than actual data
        let available_data = &data[start..];
        let mut hasher = Sha1::new();
        hasher.update(available_data);
        return hasher.finalize().to_vec();
    }

    let index_data = &data[start..end];
    let mut hasher = Sha1::new();
    hasher.update(index_data);
    hasher.finalize().to_vec()
}

/// Encrypts data using XOR with the provided key
pub fn encrypt(data: &mut [u8], key: &[u8]) {
    for (i, byte) in data.iter_mut().enumerate() {
        *byte ^= key[i % key.len()];
    }
}

/// Decrypts data using XOR with the provided key (same as encrypt for XOR)
pub fn decrypt(data: &[u8], key: &[u8]) -> Vec<u8> {
    data.iter()
        .enumerate()
        .map(|(i, &byte)| byte ^ key[i % key.len()])
        .collect()
}
