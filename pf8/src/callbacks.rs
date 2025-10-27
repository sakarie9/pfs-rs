//! Callback interfaces for progress reporting and cancellation support.

use crate::error::Result;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Progress information for extraction operations
#[derive(Debug, Clone)]
pub struct ProgressInfo {
    /// Current file being processed
    pub current_file: String,
    /// Current file index (0-based)
    pub current_file_index: usize,
    /// Total number of files to extract
    pub total_files: usize,
    /// Bytes processed for current file
    pub current_file_bytes: u64,
    /// Total bytes for current file
    pub current_file_total: u64,
    /// Total bytes processed across all files
    pub total_bytes_processed: u64,
    /// Total bytes to process across all files
    pub total_bytes: u64,
}

impl ProgressInfo {
    /// Gets the overall progress as a percentage (0.0 to 100.0)
    pub fn overall_progress(&self) -> f64 {
        if self.total_bytes == 0 {
            return 0.0;
        }
        (self.total_bytes_processed as f64 / self.total_bytes as f64) * 100.0
    }

    /// Gets the current file progress as a percentage (0.0 to 100.0)
    pub fn current_file_progress(&self) -> f64 {
        if self.current_file_total == 0 {
            return 0.0;
        }
        (self.current_file_bytes as f64 / self.current_file_total as f64) * 100.0
    }
}

/// Trait for receiving progress updates during extraction
pub trait ProgressCallback: Send {
    /// Called when progress is updated
    ///
    /// # Arguments
    /// * `progress` - Current progress information
    ///
    /// # Returns
    /// `Ok(())` to continue, or `Err(_)` to cancel the operation
    fn on_progress(&mut self, progress: &ProgressInfo) -> Result<()>;

    /// Called when starting to extract a file
    ///
    /// # Arguments
    /// * `path` - Path of the file being extracted
    /// * `file_index` - Index of the file (0-based)
    /// * `total_files` - Total number of files
    fn on_file_start(&mut self, path: &Path, file_index: usize, total_files: usize) -> Result<()> {
        let _ = (path, file_index, total_files);
        Ok(())
    }

    /// Called when a file extraction is completed
    ///
    /// # Arguments
    /// * `path` - Path of the file that was extracted
    /// * `file_index` - Index of the file (0-based)
    fn on_file_complete(&mut self, path: &Path, file_index: usize) -> Result<()> {
        let _ = (path, file_index);
        Ok(())
    }
}

/// A simple no-op callback that does nothing
pub struct NoOpCallback;

impl ProgressCallback for NoOpCallback {
    fn on_progress(&mut self, _progress: &ProgressInfo) -> Result<()> {
        Ok(())
    }
}

/// A cancellation token that can be used to cancel operations
#[derive(Debug, Clone)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    /// Creates a new cancellation token
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Cancels the operation
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    /// Checks if the operation has been cancelled
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    /// Resets the cancellation state
    pub fn reset(&self) {
        self.cancelled.store(false, Ordering::SeqCst);
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

/// A callback that supports cancellation via a token
pub struct CancellableCallback<C: ProgressCallback> {
    callback: C,
    token: CancellationToken,
}

impl<C: ProgressCallback> CancellableCallback<C> {
    /// Creates a new cancellable callback
    pub fn new(callback: C, token: CancellationToken) -> Self {
        Self { callback, token }
    }

    /// Gets a reference to the cancellation token
    pub fn token(&self) -> &CancellationToken {
        &self.token
    }
}

impl<C: ProgressCallback> ProgressCallback for CancellableCallback<C> {
    fn on_progress(&mut self, progress: &ProgressInfo) -> Result<()> {
        if self.token.is_cancelled() {
            return Err(crate::error::Error::Cancelled);
        }
        self.callback.on_progress(progress)
    }

    fn on_file_start(&mut self, path: &Path, file_index: usize, total_files: usize) -> Result<()> {
        if self.token.is_cancelled() {
            return Err(crate::error::Error::Cancelled);
        }
        self.callback.on_file_start(path, file_index, total_files)
    }

    fn on_file_complete(&mut self, path: &Path, file_index: usize) -> Result<()> {
        if self.token.is_cancelled() {
            return Err(crate::error::Error::Cancelled);
        }
        self.callback.on_file_complete(path, file_index)
    }
}
