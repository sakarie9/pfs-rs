use log::info;
use pf8::{ArchiveHandler, ControlAction};
use std::fs;
use std::path::Path;
use std::time::Instant;

/// Progress handler that collects statistics and prints progress
pub struct ProgressHandler {
    start_time: Instant,
    total_files: usize,
}

impl ProgressHandler {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            total_files: 0,
        }
    }

    pub fn print_summary(&self, total_bytes: u64, output_path: &Path) {
        let elapsed = self.start_time.elapsed();
        let elapsed_secs = elapsed.as_secs_f64();
        let speed = if elapsed_secs > 0.0 {
            total_bytes as f64 / elapsed_secs / 1024.0 / 1024.0
        } else {
            0.0
        };
        info!(
            "Done: Time: {:.2}s, Files: {}, Size: {:.2} MB, Speed: {:.2} MB/s",
            elapsed_secs,
            self.total_files,
            total_bytes as f64 / 1024.0 / 1024.0,
            speed
        );
        info!(
            "Output: {}",
            fs::canonicalize(output_path)
                .unwrap_or_else(|_| output_path.to_path_buf())
                .display()
        );
    }
}

impl Default for ProgressHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl ArchiveHandler for ProgressHandler {
    fn on_entry_started(&mut self, name: &str) -> ControlAction {
        self.total_files += 1;
        info!("Processing: {}", name);
        ControlAction::Continue
    }
}
