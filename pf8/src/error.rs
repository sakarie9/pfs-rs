//! Error types for the PF8 library.

use std::fmt;
use std::io;
use std::string::FromUtf8Error;

/// A specialized `Result` type for PF8 operations.
pub type Result<T> = std::result::Result<T, Error>;

/// The error type for PF8 operations.
#[derive(Debug)]
pub enum Error {
    /// I/O error occurred.
    Io(io::Error),
    /// Invalid PF8 file format.
    InvalidFormat(String),
    /// File not found in archive.
    FileNotFound(String),
    /// Invalid UTF-8 in file names.
    InvalidUtf8(FromUtf8Error),
    /// Encryption/decryption error.
    Crypto(String),
    /// Archive is corrupted.
    Corrupted(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(err) => write!(f, "I/O error: {err}"),
            Error::InvalidFormat(msg) => write!(f, "Invalid PF8 format: {msg}"),
            Error::FileNotFound(name) => write!(f, "File not found in archive: {name}"),
            Error::InvalidUtf8(err) => write!(f, "Invalid UTF-8 in file name: {err}"),
            Error::Crypto(msg) => write!(f, "Encryption/decryption error: {msg}"),
            Error::Corrupted(msg) => write!(f, "Archive is corrupted: {msg}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(err) => Some(err),
            Error::InvalidUtf8(err) => Some(err),
            _ => None,
        }
    }
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
