//! # PF6/PF8 Archive Library
//!
//! A Rust library for encoding and decoding PF6 and PF8 archive files.
//!
//! - **PF6**: Unencrypted format for simple file archiving
//! - **PF8**: Encrypted format with XOR encryption using SHA1-based keys
//!
//! ## Quick Start
//!
//! ### Reading PF6/PF8 Archives
//!
//! ```rust
//! use pf8::{Pf8Archive, Result, create_from_dir};
//! # use std::fs;
//! # use tempfile::TempDir;
//!
//! # fn main() -> Result<()> {
//! # let temp_dir = TempDir::new().unwrap();
//! # let archive_path = temp_dir.path().join("archive.pf8");
//! # let input_dir = temp_dir.path().join("input");
//! # fs::create_dir_all(&input_dir).unwrap();
//! # fs::write(input_dir.join("test.txt"), b"test content").unwrap();
//! # create_from_dir(&input_dir, &archive_path)?;
//! // Open an existing PF6/PF8 archive
//! let archive = Pf8Archive::open(&archive_path)?;
//!
//! // List all files in the archive
//! for entry in archive.entries()? {
//!     println!("{}: {} bytes", entry.path().display(), entry.size());
//! }
//!
//! // Extract all files to a directory
//! let output_dir = temp_dir.path().join("output");
//! archive.extract_all(&output_dir)?;
//!
//! // Extract a specific file
//! if let Some(_entry) = archive.get_entry("test.txt")? {
//!     let data = archive.read_file("test.txt")?;
//!     let output_file = temp_dir.path().join("extracted_file.txt");
//!     std::fs::write(output_file, data)?;
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ### Creating PF8 Archives
//!
//! ```rust
//! use pf8::{Pf8Builder, Result};
//! # use std::fs;
//! # use tempfile::TempDir;
//!
//! # fn main() -> Result<()> {
//! # let temp_dir = TempDir::new().unwrap();
//! # let input_dir = temp_dir.path().join("input_directory");
//! # fs::create_dir_all(&input_dir).unwrap();
//! # fs::write(input_dir.join("test.txt"), b"test content").unwrap();
//! # let single_file = temp_dir.path().join("single_file.txt");
//! # fs::write(&single_file, b"single file content").unwrap();
//! # let output_path = temp_dir.path().join("output.pf8");
//! // Create a new archive builder
//! let mut builder = Pf8Builder::new();
//!
//! // Configure encryption filters (files matching these patterns won't be encrypted)
//! builder.unencrypted_extensions(&[".txt", ".md", ".ini"]);
//!
//! // Add files and directories
//! builder.add_dir(&input_dir)?;
//! builder.add_file(&single_file)?;
//! // builder.add_file_as("config.toml", "settings/config.toml")?;
//!
//! // Write the archive to a file
//! builder.write_to_file(&output_path)?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Features
//!
//! - **Streaming Support**: Read and write archives without loading everything into memory
//! - **Dual Format Support**:
//!   - PF6: Simple unencrypted format
//!   - PF8: Encrypted format with XOR encryption and SHA1-based keys
//! - **Flexible API**: Both high-level convenience methods and low-level control
//! - **Path Handling**: Automatic conversion between system paths and internal format
//! - **Error Handling**: Comprehensive error types with detailed messages

pub mod archive;
pub mod builder;
pub mod entry;
pub mod error;
pub mod reader;
pub mod writer;

mod crypto;
mod format;
mod utils;

// Re-export main types for convenience
pub use archive::Pf8Archive;
pub use builder::Pf8Builder;
pub use entry::Pf8Entry;
pub use error::{Error, Result};
pub use format::ArchiveFormat;
pub use reader::Pf8Reader;
pub use writer::Pf8Writer;

// Re-export convenience functions
pub use archive::{create_from_dir, create_from_dir_with_patterns, extract, extract_with_patterns};

#[cfg(feature = "display")]
pub mod display;

// Re-export display functionality when feature is enabled
#[cfg(feature = "display")]
pub use display::list_archive;
