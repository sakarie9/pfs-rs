//! Example program demonstrating the PF8 library usage

use pf8::{Pf8Archive, Pf8Builder, Result, create_from_dir, extract};
use std::fs;

fn main() -> Result<()> {
    // Create a temporary directory structure for demonstration
    let temp_dir = std::env::temp_dir().join("pf8_example");
    let input_dir = temp_dir.join("input");
    let archive_path = temp_dir.join("example.pfs");
    let output_dir = temp_dir.join("output");

    // Clean up any existing files
    let _ = fs::remove_dir_all(&temp_dir);

    // Create input directory with some test files
    fs::create_dir_all(&input_dir)?;
    fs::write(
        input_dir.join("readme.txt"),
        "This is a text file that should not be encrypted.",
    )?;
    fs::write(input_dir.join("config.ini"), "[settings]\ndebug=true")?;
    fs::write(input_dir.join("data.bin"), b"\x00\x01\x02\x03\x04\x05")?;

    let subdir = input_dir.join("assets");
    fs::create_dir_all(&subdir)?;
    fs::write(subdir.join("image.png"), b"fake PNG data")?;
    fs::write(subdir.join("script.lua"), "print('Hello from Lua!')")?;

    println!("=== PF8 Library Example ===\n");

    // Example 1: Using convenience function
    println!("1. Creating archive using convenience function...");
    create_from_dir(&input_dir, &archive_path)?;
    println!("   Created: {}", archive_path.display());

    // Example 2: Reading archive information
    println!("\n2. Reading archive information...");
    let mut archive = Pf8Archive::open(&archive_path)?;
    println!("   Archive contains {} files:", archive.len());

    for entry in archive.entries() {
        println!(
            "   - {}: {} bytes (encrypted: {})",
            entry.path().display(),
            entry.size(),
            entry.is_encrypted()
        );
    }

    // Example 3: Reading specific files
    println!("\n3. Reading specific files...");
    let readme_content = archive.read_file("readme.txt")?;
    println!(
        "   readme.txt content: {}",
        String::from_utf8_lossy(&readme_content)
    );

    // Example 4: Extracting archive
    println!("\n4. Extracting archive...");
    extract(&archive_path, &output_dir)?;
    println!("   Extracted to: {}", output_dir.display());

    // Example 5: Using builder with custom encryption settings
    println!("\n5. Creating archive with custom encryption settings...");
    let archive_path2 = temp_dir.join("custom.pfs");
    let mut builder = Pf8Builder::new();

    // Configure which files should NOT be encrypted
    builder.unencrypted_extensions(&[".txt", ".ini", ".md"]);
    builder.unencrypted_patterns(&["readme"]);

    // Add files
    builder.add_dir(&input_dir)?;
    builder.write_to_file(&archive_path2)?;

    println!("   Created custom archive: {}", archive_path2.display());

    // Example 6: Reading the custom archive with appropriate patterns
    println!("\n6. Reading custom archive...");
    let custom_archive =
        Pf8Archive::open_with_patterns(&archive_path2, &[".txt", ".ini", ".md", "readme"])?;

    for entry in custom_archive.entries() {
        println!(
            "   - {}: {} bytes (encrypted: {})",
            entry.path().display(),
            entry.size(),
            entry.is_encrypted()
        );
    }

    // Example 7: Adding individual files with custom paths
    println!("\n7. Using builder to create archive with custom structure...");
    let archive_path3 = temp_dir.join("structured.pfs");
    let mut structured_builder = Pf8Builder::new();

    structured_builder.add_file_as(input_dir.join("readme.txt"), "docs/README.txt")?;
    structured_builder.add_file_as(input_dir.join("config.ini"), "config/settings.ini")?;
    structured_builder.add_dir_as(&subdir, "game/assets")?;

    structured_builder.write_to_file(&archive_path3)?;
    println!("   Created structured archive: {}", archive_path3.display());

    let structured_archive = Pf8Archive::open(&archive_path3)?;
    println!("   Structured archive contents:");
    for entry in structured_archive.entries() {
        println!("   - {}", entry.path().display());
    }

    #[cfg(feature = "display")]
    {
        println!("\n8. Pretty-printing archive contents...");
        pf8::display::list_archive(&archive_path3)?;
    }

    // Clean up
    println!("\nCleaning up temporary files...");
    let _ = fs::remove_dir_all(&temp_dir);

    println!("\n=== Example completed successfully! ===");
    Ok(())
}
