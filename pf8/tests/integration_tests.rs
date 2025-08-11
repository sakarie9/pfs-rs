//! Tests for the PF8 library

use pf8::{
    archive::{create_from_dir, extract},
    *,
};
use std::fs;
use std::path::Path;
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

/// Tests the complete integrity of pack-unpack operations
///
/// This comprehensive test verifies that all files remain identical after being
/// packed into a PF8 archive and then extracted back out. It covers:
///
/// - **Multiple file types**: Text files, binary files, empty files, large files
/// - **Complex directory structures**: Nested directories up to 3 levels deep
/// - **UTF-8 content**: Files containing Unicode characters (Chinese text)
/// - **Mixed encryption**: Some files encrypted, others unencrypted based on patterns
/// - **Special file names**: Files with spaces, dashes, and underscores
/// - **File metadata**: Verifies both content and file size integrity
///
/// The test creates a directory structure with 11 files of various types,
/// packs them into a PF8 archive with selective encryption based on file extensions,
/// extracts everything to a new location, and then performs a recursive comparison
/// to ensure perfect data integrity.
///
/// This is the most comprehensive test for data preservation in the PF8 library.
#[test]
fn test_pack_unpack_integrity() {
    let temp_dir = TempDir::new().unwrap();
    let original_dir = temp_dir.path().join("original");
    let archive_path = temp_dir.path().join("test.pfs");
    let extracted_dir = temp_dir.path().join("extracted");

    // Create a complex directory structure with various file types
    fs::create_dir_all(&original_dir).unwrap();

    // Text files (unencrypted)
    let readme_content = "This is a readme file\nwith multiple lines\nand UTF-8 content: 你好世界";
    fs::write(original_dir.join("readme.txt"), readme_content.as_bytes()).unwrap();
    fs::write(
        original_dir.join("config.ini"),
        b"[section]\nkey=value\nother_key=other_value",
    )
    .unwrap();

    // Binary files (encrypted)
    fs::write(
        original_dir.join("data.bin"),
        [0u8, 1, 2, 3, 255, 254, 253, 128, 127],
    )
    .unwrap();
    fs::write(
        original_dir.join("image.jpg"),
        b"\xFF\xD8\xFF\xE0\x00\x10JFIF\x00\x01",
    )
    .unwrap();

    // Empty file
    fs::write(original_dir.join("empty.txt"), b"").unwrap();

    // Large file with repetitive content
    let large_content = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ".repeat(1000);
    fs::write(original_dir.join("large.txt"), large_content.as_bytes()).unwrap();

    // Nested directories
    let nested_dir = original_dir.join("nested").join("deep").join("structure");
    fs::create_dir_all(&nested_dir).unwrap();
    fs::write(nested_dir.join("nested_file.txt"), b"Deep nested content").unwrap();
    fs::write(nested_dir.join("binary.dat"), [42u8; 256]).unwrap();

    // Files with special characters in names (if supported by filesystem)
    let special_dir = original_dir.join("special");
    fs::create_dir_all(&special_dir).unwrap();
    fs::write(
        special_dir.join("file with spaces.txt"),
        b"Content with spaces",
    )
    .unwrap();
    fs::write(
        special_dir.join("file-with-dashes.txt"),
        b"Content with dashes",
    )
    .unwrap();
    fs::write(
        special_dir.join("file_with_underscores.txt"),
        b"Content with underscores",
    )
    .unwrap();

    // Create archive with mixed encryption patterns
    let mut builder = Pf8Builder::new();
    builder.unencrypted_extensions(&[".txt", ".ini", ".md"]);
    builder.add_dir(&original_dir).unwrap();
    builder.write_to_file(&archive_path).unwrap();

    // Verify archive was created
    assert!(archive_path.exists());

    // Extract the archive
    let mut archive =
        Pf8Archive::open_with_patterns(&archive_path, &[".txt", ".ini", ".md"]).unwrap();
    archive.extract_all(&extracted_dir).unwrap();

    // Function to recursively compare directories
    fn compare_directories(original: &Path, extracted: &Path) -> Result<()> {
        use std::collections::HashSet;

        let original_entries: HashSet<_> = fs::read_dir(original)?
            .map(|entry| entry.unwrap().file_name())
            .collect();

        let extracted_entries: HashSet<_> = fs::read_dir(extracted)?
            .map(|entry| entry.unwrap().file_name())
            .collect();

        // Check that both directories have the same entries
        assert_eq!(original_entries, extracted_entries);

        for entry in fs::read_dir(original)? {
            let entry = entry?;
            let original_path = entry.path();
            let extracted_path = extracted.join(entry.file_name());

            if original_path.is_file() {
                // Compare file contents
                let original_content = fs::read(&original_path)?;
                let extracted_content = fs::read(&extracted_path)?;
                assert_eq!(
                    original_content,
                    extracted_content,
                    "File content mismatch: {:?}",
                    entry.file_name()
                );

                // Compare file metadata
                let original_metadata = fs::metadata(&original_path)?;
                let extracted_metadata = fs::metadata(&extracted_path)?;
                assert_eq!(
                    original_metadata.len(),
                    extracted_metadata.len(),
                    "File size mismatch: {:?}",
                    entry.file_name()
                );
            } else if original_path.is_dir() {
                // Recursively compare subdirectories
                compare_directories(&original_path, &extracted_path)?;
            }
        }

        Ok(())
    }

    // Compare the original and extracted directories
    compare_directories(&original_dir, &extracted_dir).unwrap();

    // Verify archive statistics
    assert_eq!(archive.len(), 11); // Total number of files

    // Verify specific files can be read correctly
    let readme_content = archive.read_file("readme.txt").unwrap();
    let expected_readme = "This is a readme file\nwith multiple lines\nand UTF-8 content: 你好世界";
    assert_eq!(readme_content, expected_readme.as_bytes());

    let binary_content = archive.read_file("data.bin").unwrap();
    assert_eq!(binary_content, [0u8, 1, 2, 3, 255, 254, 253, 128, 127]);

    let empty_content = archive.read_file("empty.txt").unwrap();
    assert_eq!(empty_content, b"");

    let nested_content = archive
        .read_file("nested/deep/structure/nested_file.txt")
        .unwrap();
    assert_eq!(nested_content, b"Deep nested content");

    // Test encryption status of different file types
    let readme_entry = archive.get_entry("readme.txt").unwrap();
    assert!(
        !readme_entry.is_encrypted(),
        "Text files should not be encrypted"
    );

    let binary_entry = archive.get_entry("data.bin").unwrap();
    assert!(
        binary_entry.is_encrypted(),
        "Binary files should be encrypted"
    );

    let config_entry = archive.get_entry("config.ini").unwrap();
    assert!(
        !config_entry.is_encrypted(),
        "INI files should not be encrypted"
    );
}
