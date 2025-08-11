# PF8 - Rust Library for PF6/PF8 Archive Files

[![Crates.io](https://img.shields.io/crates/v/pf8.svg)](https://crates.io/crates/pf8)
[![Documentation](https://docs.rs/pf8/badge.svg)](https://docs.rs/pf8)
[![License](https://img.shields.io/crates/l/pf8.svg)](LICENSE)

A comprehensive Rust library for encoding and decoding PF6 and PF8 archive files. This library provides both high-level convenience APIs and low-level control for working with these archive formats.

> PF6 and PF8 are proprietary archive formats for the [Artemis](https://www.ies-net.com/) galgame engine.

## Features

- **Multiple Format Support**:
  - **PF6**: Read-only support, no encryption
  - **PF8**: Full read/write support with XOR encryption
- **Streaming Support**: Read and write archives without loading everything into memory
- **Built-in Encryption**: XOR encryption with SHA1-based keys (PF8 only)
- **Flexible API**: Both high-level convenience methods and low-level control
- **Path Handling**: Automatic conversion between system paths and archive internal format
- **Comprehensive Error Handling**: Detailed error types with helpful messages
- **Optional Display Features**: Pretty-printed archive listings (requires `display` feature)

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
pf8 = "0.1"

# Without display features (pretty-printed tables)
pf8 = { version = "0.1", default-features = false }
```

### Convenience Functions

For simple operations, use the convenience functions:

```rust
use pf8::{extract, create_from_dir, Result};

fn main() -> Result<()> {
    // Extract an archive
    extract("root.pfs", "output_directory")?;

    // Create an archive from a directory
    create_from_dir("input_directory", "new_archive.pfs")?;

    Ok(())
}
```

### Reading PF8 Archives

```rust
use pf8::{Pf8Archive, Result};

fn main() -> Result<()> {
    // Open an existing PF8 archive
    let mut archive = Pf8Archive::open("root.pfs")?;

    // List all files in the archive
    for entry in archive.entries() {
        println!("{}: {} bytes", entry.path().display(), entry.size());
    }

    // Extract all files to a directory
    archive.extract_all("output_dir")?;

    // Extract a specific file
    if let Some(_entry) = archive.get_entry("system/table/list_windows.tbl") {
        let data = archive.read_file("system/table/list_windows.tbl")?;
        std::fs::write("extracted_list_windows.tbl", data)?;
    }

    Ok(())
}
```

### Creating PF8 Archives

```rust
use pf8::{Pf8Builder, Result};

fn main() -> Result<()> {
    // Create a new archive builder
    let mut builder = Pf8Builder::new();

    // Configure encryption filters (files matching these patterns won't be encrypted)
    // Ignore this will use default unencrypted lists
    // builder.unencrypted_extensions(&[".mp4", ".flv"]);

    // Add files and directories
    builder.add_dir("scripts")?;
    builder.add_file("single_file.txt")?;
    builder.add_file_as("list_windows.tbl", "system/table/list_windows.tbl")?;

    // Write the archive to a file
    builder.write_to_file("root.pfs")?;

    Ok(())
}
```

### Display Features

With the `display` feature enabled, you can pretty-print archive contents:

```rust
use pf8::display::list_archive;

fn main() -> pf8::Result<()> {
    list_archive("root.pfs")?;
    Ok(())
}
```

This will output a formatted table like:

```text
archive.pfs

| File              | Size      |
|-------------------|-----------|
| config/game.ini   | 1.2 KB    |
| image/image1.png  | 45.6 MB   |
| scripts/main.ast  | 3.4 KB    |

Total: 3 files, Total size: 45.6 MB
```

## Advanced Usage

### Low-level Reader API

```rust
use pf8::{Pf8Reader, Result};

fn main() -> Result<()> {
    let mut reader = Pf8Reader::open("root.pfs")?;
    
    for entry in reader.entries() {
        println!("File: {}", entry.path().display());
        println!("Size: {} bytes", entry.size());
        println!("Encrypted: {}", entry.is_encrypted());
        
        // Read file data
        let data = reader.read_file(entry.path())?;
        
        // Process data...
    }
    
    Ok(())
}
```

### Custom Encryption Patterns

```rust
use pf8::{Pf8Archive, Pf8Builder, Result};

fn main() -> Result<()> {
    // When reading, specify which files should be unencrypted
    let unencrypted_patterns = &[".mp4", ".flv"];
    let archive = Pf8Archive::open_with_patterns("root.pfs", unencrypted_patterns)?;
    
    // When creating, specify patterns for unencrypted files
    let mut builder = Pf8Builder::new();
    builder.unencrypted_patterns(&[".mp4", ".flv"]);
    builder.add_dir("src")?;
    builder.write_to_file("root.pfs")?;
    
    Ok(())
}
```

### Streaming Operations

For large archives, you can use streaming operations to avoid loading everything into memory:

```rust
use pf8::{Pf8Reader, Result};

fn extract_large_archive(archive_path: &str, output_dir: &str) -> Result<()> {
    let mut reader = Pf8Reader::open(archive_path)?;
    
    for entry in reader.entries() {
        let output_path = std::path::Path::new(output_dir).join(entry.path());
        
        // Create parent directories
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        // Stream file data directly to disk
        use std::fs::File;
        use std::io::Write;
        
        let mut output_file = File::create(&output_path)?;
        reader.read_file_streaming(entry.path(), |chunk| {
            output_file.write_all(chunk)?;
            Ok(())
        })?;
    }
    
    Ok(())
}
```

## Error Handling

The library provides comprehensive error types:

```rust
use pf8::{Error, Result};

fn handle_errors() -> Result<()> {
    match pf8::extract("root.pfs", "output") {
        Ok(()) => println!("Extraction successful"),
        Err(Error::Io(e)) => eprintln!("I/O error: {}", e),
        Err(Error::InvalidFormat(msg)) => eprintln!("Invalid format: {}", msg),
        Err(Error::FileNotFound(name)) => eprintln!("File not found: {}", name),
        Err(Error::Corrupted(msg)) => eprintln!("Archive corrupted: {}", msg),
        Err(e) => eprintln!("Other error: {}", e),
    }
    
    Ok(())
}
```

## PF8 Format Details

The PF8 format is a custom archive format with the following features:

- **Magic Number**: Files start with "pf8" (3 bytes)
- **Index Structure**: Contains file names, offsets, and sizes
- **XOR Encryption**: File contents are encrypted using XOR with SHA1-derived keys
- **Path Format**: Uses backslash separators internally
- **Little-Endian**: All multi-byte integers are stored in little-endian format

## Performance Considerations

- Streaming operations for files read/write
- The `display` feature adds dependencies - disable if not needed
- Encryption/decryption is performed in-memory

## License

This project is licensed under the MIT license - see the [LICENSE](LICENSE) file for details.
