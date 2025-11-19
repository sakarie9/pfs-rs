//! Callback interfaces for progress reporting and cancellation support.
//!
//! This module provides a unified event-driven callback interface that supports
//! both packing and unpacking operations with comprehensive lifecycle events.

use std::fmt;

/// Operation type: Pack or Unpack
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationType {
    /// Packing/Compressing files into an archive
    Pack,
    /// Unpacking/Extracting files from an archive
    Unpack,
}

impl fmt::Display for OperationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OperationType::Pack => write!(f, "Pack"),
            OperationType::Unpack => write!(f, "Unpack"),
        }
    }
}

/// Progress information for archive operations
#[derive(Debug, Clone)]
pub struct ProgressInfo {
    /// Bytes processed so far
    pub processed_bytes: u64,
    /// Total bytes to process (None if unknown, e.g., during packing)
    pub total_bytes: Option<u64>,
    /// Number of files processed (1-based, i.e., currently processing the Nth file)
    pub processed_files: usize,
    /// Total number of files to process (None if unknown)
    pub total_files: Option<usize>,
    /// Current file being processed (name or path)
    pub current_file: String,
}

impl ProgressInfo {
    /// Gets the overall progress as a percentage (0.0 to 100.0)
    /// Returns None if total_bytes is unknown
    pub fn overall_progress(&self) -> Option<f64> {
        self.total_bytes.map(|total| {
            if total == 0 {
                0.0
            } else {
                (self.processed_bytes as f64 / total as f64) * 100.0
            }
        })
    }

    /// Gets the file progress as a percentage (0.0 to 100.0)
    /// Returns None if total_files is unknown
    pub fn file_progress(&self) -> Option<f64> {
        self.total_files.map(|total| {
            if total == 0 {
                0.0
            } else {
                (self.processed_files as f64 / total as f64) * 100.0
            }
        })
    }
}

/// Control action returned by callback to control operation flow
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlAction {
    /// Continue the operation
    Continue,
    /// Abort the operation immediately
    Abort,
}

/// Archive-specific errors that can occur during operations
#[derive(Debug, Clone)]
pub enum ArchiveError {
    /// I/O error occurred
    IoError(String),
    /// Archive format error
    FormatError(String),
    /// File or entry not found
    NotFound(String),
    /// Compression/Decompression error
    CompressionError(String),
    /// Permission denied
    PermissionDenied(String),
    /// Other error
    Other(String),
}

impl fmt::Display for ArchiveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArchiveError::IoError(msg) => write!(f, "I/O Error: {}", msg),
            ArchiveError::FormatError(msg) => write!(f, "Format Error: {}", msg),
            ArchiveError::NotFound(msg) => write!(f, "Not Found: {}", msg),
            ArchiveError::CompressionError(msg) => write!(f, "Compression Error: {}", msg),
            ArchiveError::PermissionDenied(msg) => write!(f, "Permission Denied: {}", msg),
            ArchiveError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for ArchiveError {}

/// Unified event enum covering all lifecycle stages of archive operations
#[derive(Debug, Clone)]
pub enum ArchiveEvent {
    /// Task started (indicates whether it's Pack or Unpack)
    Started(OperationType),
    /// Started processing a specific file/entry
    EntryStarted(String),
    /// Periodic progress update
    Progress(ProgressInfo),
    /// Finished processing a specific file/entry
    EntryFinished(String),
    /// Non-fatal warning (e.g., file skipped due to lock)
    Warning(String),
    /// Task completed successfully
    Finished,
}

impl fmt::Display for ArchiveEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArchiveEvent::Started(op) => write!(f, "Started: {}", op),
            ArchiveEvent::EntryStarted(name) => write!(f, "Entry Started: {}", name),
            ArchiveEvent::Progress(info) => {
                write!(
                    f,
                    "Progress: {} ({}/",
                    info.current_file, info.processed_files
                )?;
                if let Some(total) = info.total_files {
                    write!(f, "{}", total)?;
                } else {
                    write!(f, "?")?;
                }
                write!(f, " files, {} bytes", info.processed_bytes)?;
                if let Some(total) = info.total_bytes {
                    write!(f, "/{} bytes", total)?;
                }
                write!(f, ")")
            }
            ArchiveEvent::EntryFinished(name) => write!(f, "Entry Finished: {}", name),
            ArchiveEvent::Warning(msg) => write!(f, "Warning: {}", msg),
            ArchiveEvent::Finished => write!(f, "Finished"),
        }
    }
}

/// Trait for handling archive operation events
///
/// This unified interface supports both packing and unpacking operations
/// through a single event-driven callback mechanism.
///
/// # Example
///
/// ```rust,ignore
/// struct MyHandler;
///
/// impl ArchiveHandler for MyHandler {
///     fn on_event(&mut self, event: &ArchiveEvent) -> ControlAction {
///         match event {
///             ArchiveEvent::Started(op) => {
///                 println!("Operation started: {}", op);
///                 ControlAction::Continue
///             }
///             ArchiveEvent::Progress(info) => {
///                 println!("Processing: {}", info.current_file);
///                 // Abort if some condition is met
///                 if should_cancel() {
///                     return ControlAction::Abort;
///                 }
///                 ControlAction::Continue
///             }
///             ArchiveEvent::Finished => {
///                 println!("Operation completed");
///                 ControlAction::Continue
///             }
///             _ => ControlAction::Continue,
///         }
///     }
/// }
/// ```
pub trait ArchiveHandler: Send {
    /// Called when an archive event occurs
    ///
    /// # Arguments
    /// * `event` - The event that occurred
    ///
    /// # Returns
    /// `ControlAction::Continue` to proceed, or `ControlAction::Abort` to cancel
    fn on_event(&mut self, event: &ArchiveEvent) -> ControlAction;
}

/// A no-op handler that does nothing and always continues
pub struct NoOpHandler;

impl ArchiveHandler for NoOpHandler {
    fn on_event(&mut self, _event: &ArchiveEvent) -> ControlAction {
        ControlAction::Continue
    }
}
