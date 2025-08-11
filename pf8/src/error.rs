//! Error types for the PF8 library.

use std::io;
use std::string::FromUtf8Error;

/// A specialized `Result` type for PF8 operations.
pub type Result<T> = std::result::Result<T, Error>;

/// The error type for PF8 operations.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// I/O error occurred.
    #[error("I/O error: {0}")]
    Io(io::Error),
    /// Invalid PF8 file format.
    #[error("Invalid PF8 format: {0}")]
    InvalidFormat(String),
    /// File not found in archive.
    #[error("File not found in archive: {0}")]
    FileNotFound(String),
    /// Invalid UTF-8 in file names.
    #[error("Invalid UTF-8 in file name: {0}")]
    InvalidUtf8(FromUtf8Error),
    /// Encryption/decryption error.
    #[error("Encryption/decryption error: {0}")]
    Crypto(String),
    /// Archive is corrupted.
    #[error("Archive is corrupted: {0}")]
    Corrupted(String),
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::Io(err)
    }
}

impl From<FromUtf8Error> for Error {
    fn from(err: FromUtf8Error) -> Self {
        Error::InvalidUtf8(err)
    }
}

impl From<walkdir::Error> for Error {
    fn from(err: walkdir::Error) -> Self {
        Error::Io(err.into())
    }
}
