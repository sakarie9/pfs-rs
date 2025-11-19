//! Example demonstrating the new unified ArchiveHandler interface with event-driven callbacks.

use pf8::{ArchiveEvent, ArchiveHandler, ControlAction, Pf8Archive, Result};
use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// A simple handler that prints progress to stdout
struct SimpleProgressHandler {
    last_percentage: f64,
}

impl SimpleProgressHandler {
    fn new() -> Self {
        Self {
            last_percentage: 0.0,
        }
    }
}

impl ArchiveHandler for SimpleProgressHandler {
    fn on_event(&mut self, event: &ArchiveEvent) -> ControlAction {
        match event {
            ArchiveEvent::Started(op_type) => {
                println!("🚀 Operation started: {}", op_type);
                ControlAction::Continue
            }
            ArchiveEvent::EntryStarted(name) => {
                println!("  📄 Processing: {}", name);
                ControlAction::Continue
            }
            ArchiveEvent::Progress(info) => {
                if let Some(percentage) = info.overall_progress() {
                    // Only print when progress changes by at least 1%
                    if (percentage - self.last_percentage).abs() >= 1.0 {
                        print!(
                            "\r  Progress: {:.1}% ({}/",
                            percentage, info.processed_files
                        );
                        if let Some(total) = info.total_files {
                            print!("{}", total);
                        } else {
                            print!("?");
                        }
                        print!(" files, {} bytes", info.processed_bytes);
                        if let Some(total) = info.total_bytes {
                            print!("/{} bytes", total);
                        }
                        print!(")");
                        std::io::stdout().flush().unwrap();
                        self.last_percentage = percentage;
                    }
                }
                ControlAction::Continue
            }
            ArchiveEvent::EntryFinished(name) => {
                println!("\n  ✓ Completed: {}", name);
                ControlAction::Continue
            }
            ArchiveEvent::Warning(msg) => {
                println!("  ⚠ Warning: {}", msg);
                ControlAction::Continue
            }
            ArchiveEvent::Finished => {
                println!("\n✅ Operation finished successfully!");
                ControlAction::Continue
            }
        }
    }
}

/// A handler that can be cancelled externally
struct CancellableHandler {
    cancel_flag: Arc<AtomicBool>,
    inner: SimpleProgressHandler,
}

impl CancellableHandler {
    fn new(cancel_flag: Arc<AtomicBool>) -> Self {
        Self {
            cancel_flag,
            inner: SimpleProgressHandler::new(),
        }
    }
}

impl ArchiveHandler for CancellableHandler {
    fn on_event(&mut self, event: &ArchiveEvent) -> ControlAction {
        // Check if cancellation was requested
        if self.cancel_flag.load(Ordering::SeqCst) {
            println!("\n⚠ Cancellation requested!");
            return ControlAction::Abort;
        }

        // Delegate to inner handler
        self.inner.on_event(event)
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
    let mut handler = SimpleProgressHandler::new();
    archive.extract_all_with_progress(&output_dir, &mut handler)?;

    // Clean up output directory for next example
    std::fs::remove_dir_all(&output_dir).unwrap();

    // Example 2: Extract with cancellation support
    println!("\n=== Example 2: Extract with cancellation (simulated) ===");
    let mut archive = Pf8Archive::open(&archive_path)?;

    // Create a cancellation flag
    let cancel_flag = Arc::new(AtomicBool::new(false));
    let cancel_flag_clone = cancel_flag.clone();

    // Simulate cancellation after a short delay (in real use, this would be triggered by user input)
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(100));
        cancel_flag_clone.store(true, Ordering::SeqCst);
        println!("\n⚠ Cancellation signal sent!");
    });

    let mut handler = CancellableHandler::new(cancel_flag.clone());
    match archive.extract_all_with_progress(&output_dir, &mut handler) {
        Ok(_) => {
            println!("\n✓ Extraction completed");
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
