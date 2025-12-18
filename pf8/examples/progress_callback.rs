//! Example demonstrating the new unified ArchiveHandler interface with event-driven callbacks.
//!
//! This example shows the recommended approach: override only the specific event methods
//! you care about, rather than matching on the generic on_event method.

use pf8::{ArchiveHandler, ControlAction, OperationType, Pf8Archive, ProgressInfo, Result};
use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// A comprehensive handler that handles all events
struct VerboseProgressHandler {
    last_percentage: f64,
}

impl VerboseProgressHandler {
    fn new() -> Self {
        Self {
            last_percentage: 0.0,
        }
    }
}

impl ArchiveHandler for VerboseProgressHandler {
    fn on_started(&mut self, op_type: OperationType) -> ControlAction {
        println!("Operation started: {}", op_type);
        ControlAction::Continue
    }

    fn on_entry_started(&mut self, name: &str) -> ControlAction {
        println!("  Processing: {}", name);
        ControlAction::Continue
    }

    fn on_progress(&mut self, info: &ProgressInfo) -> ControlAction {
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

    fn on_entry_finished(&mut self, name: &str) -> ControlAction {
        println!("\n  Completed: {}", name);
        ControlAction::Continue
    }

    fn on_warning(&mut self, message: &str) -> ControlAction {
        println!("  Warning: {}", message);
        ControlAction::Continue
    }

    fn on_finished(&mut self) -> ControlAction {
        println!("\nOperation finished successfully!");
        ControlAction::Continue
    }
}

/// A handler that can be cancelled externally
struct CancellableHandler {
    cancel_flag: Arc<AtomicBool>,
    inner: VerboseProgressHandler,
}

impl CancellableHandler {
    fn new(cancel_flag: Arc<AtomicBool>) -> Self {
        Self {
            cancel_flag,
            inner: VerboseProgressHandler::new(),
        }
    }
}

impl ArchiveHandler for CancellableHandler {
    fn on_started(&mut self, op_type: OperationType) -> ControlAction {
        if self.cancel_flag.load(Ordering::SeqCst) {
            println!("\nCancellation requested!");
            return ControlAction::Abort;
        }
        self.inner.on_started(op_type)
    }

    fn on_entry_started(&mut self, name: &str) -> ControlAction {
        if self.cancel_flag.load(Ordering::SeqCst) {
            println!("\nCancellation requested!");
            return ControlAction::Abort;
        }
        self.inner.on_entry_started(name)
    }

    fn on_progress(&mut self, info: &ProgressInfo) -> ControlAction {
        if self.cancel_flag.load(Ordering::SeqCst) {
            println!("\nCancellation requested!");
            return ControlAction::Abort;
        }
        self.inner.on_progress(info)
    }

    fn on_entry_finished(&mut self, name: &str) -> ControlAction {
        if self.cancel_flag.load(Ordering::SeqCst) {
            println!("\nCancellation requested!");
            return ControlAction::Abort;
        }
        self.inner.on_entry_finished(name)
    }

    fn on_warning(&mut self, message: &str) -> ControlAction {
        if self.cancel_flag.load(Ordering::SeqCst) {
            println!("\nCancellation requested!");
            return ControlAction::Abort;
        }
        self.inner.on_warning(message)
    }

    fn on_finished(&mut self) -> ControlAction {
        self.inner.on_finished()
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

    // Example 1: Extract with verbose progress reporting
    println!("\n=== Example 1: Extract with progress reporting ===");
    let mut archive = Pf8Archive::open(&archive_path)?;
    let mut handler = VerboseProgressHandler::new();
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
        println!("\nCancellation signal sent!");
    });

    let mut handler = CancellableHandler::new(cancel_flag.clone());
    match archive.extract_all_with_progress(&output_dir, &mut handler) {
        Ok(_) => {
            println!("\nExtraction completed");
        }
        Err(pf8::Error::Cancelled) => {
            println!("Extraction was successfully cancelled");
        }
        Err(e) => {
            println!("Error: {}", e);
            return Err(e);
        }
    }

    println!("\n=== All examples completed ===");
    Ok(())
}
