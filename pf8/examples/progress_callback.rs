//! Example demonstrating progress callbacks and cancellation support for extraction.

use pf8::{CancellationToken, Pf8Archive, ProgressCallback, ProgressInfo, Result};
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// A simple progress callback that prints progress to stdout
struct SimpleProgressCallback {
    last_percentage: f64,
}

impl SimpleProgressCallback {
    fn new() -> Self {
        Self {
            last_percentage: 0.0,
        }
    }
}

impl ProgressCallback for SimpleProgressCallback {
    fn on_progress(&mut self, progress: &ProgressInfo) -> Result<()> {
        let percentage = progress.overall_progress();
        
        // Only print when progress changes by at least 1%
        if (percentage - self.last_percentage).abs() >= 1.0 {
            print!(
                "\rProgress: {:.1}% ({}/{} files, {}/{} bytes)",
                percentage,
                progress.current_file_index + 1,
                progress.total_files,
                progress.total_bytes_processed,
                progress.total_bytes
            );
            std::io::stdout().flush().unwrap();
            self.last_percentage = percentage;
        }
        
        Ok(())
    }

    fn on_file_start(&mut self, path: &Path, file_index: usize, total_files: usize) -> Result<()> {
        println!(
            "\n[{}/{}] Starting extraction: {}",
            file_index + 1,
            total_files,
            path.display()
        );
        Ok(())
    }

    fn on_file_complete(&mut self, path: &Path, _file_index: usize) -> Result<()> {
        println!("  ✓ Completed: {}", path.display());
        Ok(())
    }
}

/// A callback that can be cancelled externally
struct CancellableProgressCallback {
    token: CancellationToken,
    inner: SimpleProgressCallback,
}

impl CancellableProgressCallback {
    fn new(token: CancellationToken) -> Self {
        Self {
            token,
            inner: SimpleProgressCallback::new(),
        }
    }
}

impl ProgressCallback for CancellableProgressCallback {
    fn on_progress(&mut self, progress: &ProgressInfo) -> Result<()> {
        if self.token.is_cancelled() {
            println!("\n⚠ Extraction cancelled by user!");
            return Err(pf8::Error::Cancelled);
        }
        self.inner.on_progress(progress)
    }

    fn on_file_start(&mut self, path: &Path, file_index: usize, total_files: usize) -> Result<()> {
        if self.token.is_cancelled() {
            return Err(pf8::Error::Cancelled);
        }
        self.inner.on_file_start(path, file_index, total_files)
    }

    fn on_file_complete(&mut self, path: &Path, file_index: usize) -> Result<()> {
        if self.token.is_cancelled() {
            return Err(pf8::Error::Cancelled);
        }
        self.inner.on_file_complete(path, file_index)
    }
}

fn main() -> Result<()> {
    // For this example, we'll create a test archive first
    let temp_dir = tempfile::TempDir::new().unwrap();
    let archive_path = temp_dir.path().join("test.pf8");
    let input_dir = temp_dir.path().join("input");
    let output_dir = temp_dir.path().join("output");
    
    // Create some test files
    std::fs::create_dir_all(&input_dir).unwrap();
    for i in 0..10 {
        let file_path = input_dir.join(format!("file_{}.txt", i));
        let content = format!("Test content for file {}\n", i).repeat(1000);
        std::fs::write(file_path, content).unwrap();
    }
    
    // Create the archive
    println!("Creating test archive...");
    pf8::create_from_dir(&input_dir, &archive_path)?;
    
    // Example 1: Extract with simple progress reporting
    println!("\n=== Example 1: Extract with progress reporting ===");
    let mut archive = Pf8Archive::open(&archive_path)?;
    let mut callback = SimpleProgressCallback::new();
    archive.extract_all_with_progress(&output_dir, &mut callback)?;
    println!("\n✓ Extraction completed successfully!");
    
    // Clean up output directory for next example
    std::fs::remove_dir_all(&output_dir).unwrap();
    
    // Example 2: Extract with cancellation support
    println!("\n=== Example 2: Extract with cancellation (simulated) ===");
    let mut archive = Pf8Archive::open(&archive_path)?;
    let token = CancellationToken::new();
    let token_clone = token.clone();
    
    // Simulate cancellation after a short delay (in real use, this would be triggered by user input)
    let cancel_flag = Arc::new(AtomicBool::new(false));
    let cancel_flag_clone = cancel_flag.clone();
    
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(100));
        if !cancel_flag_clone.load(Ordering::SeqCst) {
            token_clone.cancel();
            println!("\n⚠ Cancellation triggered!");
        }
    });
    
    let mut callback = CancellableProgressCallback::new(token);
    match archive.extract_all_with_progress(&output_dir, &mut callback) {
        Ok(_) => {
            cancel_flag.store(true, Ordering::SeqCst);
            println!("\n✓ Extraction completed before cancellation");
        }
        Err(pf8::Error::Cancelled) => {
            println!("✓ Extraction was successfully cancelled");
        }
        Err(e) => {
            println!("✗ Error: {}", e);
            return Err(e);
        }
    }
    
    println!("\n=== All examples completed ===");
    Ok(())
}
