//! Tests for the PF8 library

use pf8::{
    archive::{create_from_dir, extract},
    *,
};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_create_and_read_simple_archive() {
    let temp_dir = TempDir::new().unwrap();
    let input_dir = temp_dir.path().join("input");
    let archive_path = temp_dir.path().join("test.pfs");
    let output_dir = temp_dir.path().join("output");

    // Create input directory with some test files
    fs::create_dir_all(&input_dir).unwrap();
    fs::write(input_dir.join("file1.txt"), b"Hello, World!").unwrap();
    fs::write(input_dir.join("file2.bin"), b"\x00\x01\x02\x03").unwrap();

    let subdir = input_dir.join("subdir");
    fs::create_dir_all(&subdir).unwrap();
    fs::write(subdir.join("file3.txt"), b"Nested file content").unwrap();

    // Create archive using builder
    let mut builder = Pf8Builder::new();
    builder.unencrypted_extensions(&[".txt"]);
    builder.add_dir(&input_dir).unwrap();
    builder.write_to_file(&archive_path).unwrap();

    // Verify archive was created
    assert!(archive_path.exists());

    // Read archive and verify contents
    let mut archive = Pf8Archive::open_with_patterns(&archive_path, &[".txt"]).unwrap();

    assert_eq!(archive.len(), 3);
    assert!(archive.contains("file1.txt"));
    assert!(archive.contains("file2.bin"));
    assert!(archive.contains("subdir/file3.txt"));

    // Test file content reading
    let content1 = archive.read_file("file1.txt").unwrap();
    assert_eq!(content1, b"Hello, World!");

    let content2 = archive.read_file("file2.bin").unwrap();
    assert_eq!(content2, b"\x00\x01\x02\x03");

    let content3 = archive.read_file("subdir/file3.txt").unwrap();
    assert_eq!(content3, b"Nested file content");

    // Test extraction
    archive.extract_all(&output_dir).unwrap();

    assert_eq!(
        fs::read(output_dir.join("file1.txt")).unwrap(),
        b"Hello, World!"
    );
    assert_eq!(
        fs::read(output_dir.join("file2.bin")).unwrap(),
        b"\x00\x01\x02\x03"
    );
    assert_eq!(
        fs::read(output_dir.join("subdir/file3.txt")).unwrap(),
        b"Nested file content"
    );
}

#[test]
fn test_convenience_functions() {
    let temp_dir = TempDir::new().unwrap();
    let input_dir = temp_dir.path().join("input");
    let archive_path = temp_dir.path().join("test.pfs");
    let output_dir = temp_dir.path().join("output");

    // Create input directory
    fs::create_dir_all(&input_dir).unwrap();
    fs::write(input_dir.join("test.txt"), b"Test content").unwrap();

    // Test convenience creation
    create_from_dir(&input_dir, &archive_path).unwrap();
    assert!(archive_path.exists());

    // Test convenience extraction
    extract(&archive_path, &output_dir).unwrap();
    assert_eq!(
        fs::read(output_dir.join("test.txt")).unwrap(),
        b"Test content"
    );
}

#[test]
fn test_builder_add_file_as() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("input.txt");
    let archive_path = temp_dir.path().join("test.pfs");

    fs::write(&input_file, b"File content").unwrap();

    let mut builder = Pf8Builder::new();
    builder
        .add_file_as(&input_file, "custom/path/file.txt")
        .unwrap();
    builder.write_to_file(&archive_path).unwrap();

    let mut archive = Pf8Archive::open(&archive_path).unwrap();
    assert!(archive.contains("custom/path/file.txt"));

    let content = archive.read_file("custom/path/file.txt").unwrap();
    assert_eq!(content, b"File content");
}

#[test]
fn test_encryption_patterns() {
    let temp_dir = TempDir::new().unwrap();
    let input_dir = temp_dir.path().join("input");
    let archive_path = temp_dir.path().join("test.pfs");

    fs::create_dir_all(&input_dir).unwrap();
    fs::write(input_dir.join("config.txt"), b"Config data").unwrap();
    fs::write(input_dir.join("data.bin"), b"Binary data").unwrap();

    // Create archive with .txt files unencrypted
    let mut builder = Pf8Builder::new();
    builder.unencrypted_extensions(&[".txt"]);
    builder.add_dir(&input_dir).unwrap();
    builder.write_to_file(&archive_path).unwrap();

    // Open with same patterns
    let mut archive = Pf8Archive::open_with_patterns(&archive_path, &[".txt"]).unwrap();

    // Verify we can read both files
    let config_content = archive.read_file("config.txt").unwrap();
    assert_eq!(config_content, b"Config data");

    let data_content = archive.read_file("data.bin").unwrap();
    assert_eq!(data_content, b"Binary data");
}

#[test]
fn test_empty_archive() {
    let temp_dir = TempDir::new().unwrap();
    let archive_path = temp_dir.path().join("empty.pfs");

    // Try to create empty archive (should fail)
    let builder = Pf8Builder::new();
    let result = builder.write_to_file(&archive_path);
    assert!(result.is_err());
}

#[test]
fn test_file_not_found_error() {
    let temp_dir = TempDir::new().unwrap();
    let nonexistent = temp_dir.path().join("nonexistent.txt");

    let mut builder = Pf8Builder::new();
    let result = builder.add_file(&nonexistent);
    assert!(result.is_err());
}

#[test]
fn test_reader_low_level_api() {
    let temp_dir = TempDir::new().unwrap();
    let input_dir = temp_dir.path().join("input");
    let archive_path = temp_dir.path().join("test.pfs");

    fs::create_dir_all(&input_dir).unwrap();
    fs::write(input_dir.join("test.txt"), b"Test content").unwrap();

    // Create archive
    create_from_dir(&input_dir, &archive_path).unwrap();

    // Test low-level reader API
    let mut reader = Pf8Reader::open(&archive_path).unwrap();

    assert_eq!(reader.len(), 1);
    assert!(!reader.is_empty());

    let entry = reader.get_entry("test.txt").unwrap();
    assert_eq!(entry.size(), 12);
    assert_eq!(entry.file_name().unwrap(), "test.txt");

    let content = reader.read_file("test.txt").unwrap();
    assert_eq!(content, b"Test content");
}
