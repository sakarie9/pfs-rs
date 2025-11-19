//! Callback interfaces for progress reporting and cancellation support.
//!
//! This module provides a unified event-driven callback interface that supports
//! both packing and unpacking operations with comprehensive lifecycle events.
//!
//! # Performance Optimization
//!
//! The [`ArchiveHandler`] trait provides **default implementations** for all event methods.
//! This means you only need to override the events you care about, avoiding unnecessary
//! overhead for unhandled events.
//!
//! ## Zero-Cost Abstraction
//!
//! When you only override specific methods (e.g., `on_progress`), the Rust compiler
//! can optimize away the overhead of events you don't handle. The default implementations
//! are inline and trivial, so they compile to essentially zero overhead.
//!
//! ### Example: Progress-only handler
//!
//! ```rust,ignore
//! struct MyHandler;
//!
//! impl ArchiveHandler for MyHandler {
//!     // Only override what you need
//!     fn on_progress(&mut self, info: &ProgressInfo) -> ControlAction {
//!         println!("Progress: {:.1}%", info.overall_progress().unwrap_or(0.0));
//!         ControlAction::Continue
//!     }
//!     // All other events (on_started, on_entry_started, etc.) use default no-op implementation
//! }
//! ```
//!
//! In this example, only progress events incur any overhead. Events like `on_entry_started`
//! and `on_entry_finished` still fire, but they immediately return `Continue` with minimal cost.

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
/// **Performance Note**: All methods have default implementations that return
/// `ControlAction::Continue`. You only need to override the events you care about,
/// avoiding unnecessary overhead for unhandled events.
///
/// # Example: Handling only progress events
///
/// ```rust,ignore
/// struct ProgressOnlyHandler;
///
/// impl ArchiveHandler for ProgressOnlyHandler {
///     fn on_progress(&mut self, info: &ProgressInfo) -> ControlAction {
///         println!("Progress: {:.1}%", info.overall_progress().unwrap_or(0.0));
///         ControlAction::Continue
///     }
///     // All other events use default implementation (no-op)
/// }
/// ```
///
/// # Example: Handling multiple events
///
/// ```rust,ignore
/// struct CustomHandler {
///     cancel_flag: bool,
/// }
///
/// impl ArchiveHandler for CustomHandler {
///     fn on_started(&mut self, op: OperationType) -> ControlAction {
///         println!("Started: {}", op);
///         ControlAction::Continue
///     }
///
///     fn on_progress(&mut self, info: &ProgressInfo) -> ControlAction {
///         if self.cancel_flag {
///             return ControlAction::Abort;
///         }
///         println!("Processing: {}", info.current_file);
///         ControlAction::Continue
///     }
///
///     fn on_finished(&mut self) -> ControlAction {
///         println!("Completed!");
///         ControlAction::Continue
///     }
/// }
/// ```
pub trait ArchiveHandler: Send {
    /// Called when the archive operation starts
    ///
    /// # Arguments
    /// * `op_type` - The type of operation (Pack or Unpack)
    ///
    /// # Returns
    /// `ControlAction::Continue` to proceed, or `ControlAction::Abort` to cancel
    #[allow(unused_variables)]
    fn on_started(&mut self, op_type: OperationType) -> ControlAction {
        ControlAction::Continue
    }

    /// Called when starting to process a specific file/entry
    ///
    /// # Arguments
    /// * `name` - The name or path of the entry being processed
    ///
    /// # Returns
    /// `ControlAction::Continue` to proceed, or `ControlAction::Abort` to cancel
    #[allow(unused_variables)]
    fn on_entry_started(&mut self, name: &str) -> ControlAction {
        ControlAction::Continue
    }

    /// Called periodically to report progress
    ///
    /// This is where most of the performance-critical updates happen.
    /// Only override this if you need progress updates.
    ///
    /// # Arguments
    /// * `info` - Current progress information
    ///
    /// # Returns
    /// `ControlAction::Continue` to proceed, or `ControlAction::Abort` to cancel
    #[allow(unused_variables)]
    fn on_progress(&mut self, info: &ProgressInfo) -> ControlAction {
        ControlAction::Continue
    }

    /// Called when an entry has been processed successfully
    ///
    /// # Arguments
    /// * `name` - The name or path of the entry that was processed
    ///
    /// # Returns
    /// `ControlAction::Continue` to proceed, or `ControlAction::Abort` to cancel
    #[allow(unused_variables)]
    fn on_entry_finished(&mut self, name: &str) -> ControlAction {
        ControlAction::Continue
    }

    /// Called when a non-fatal warning occurs
    ///
    /// # Arguments
    /// * `message` - The warning message
    ///
    /// # Returns
    /// `ControlAction::Continue` to proceed, or `ControlAction::Abort` to cancel
    #[allow(unused_variables)]
    fn on_warning(&mut self, message: &str) -> ControlAction {
        ControlAction::Continue
    }

    /// Called when the operation completes successfully
    ///
    /// # Returns
    /// `ControlAction::Continue` (ignored) or `ControlAction::Abort` (ignored)
    fn on_finished(&mut self) -> ControlAction {
        ControlAction::Continue
    }

    /// Internal dispatcher method - do not override
    ///
    /// This method dispatches events to the appropriate handler methods.
    /// Users should override the specific event methods instead.
    #[doc(hidden)]
    fn on_event(&mut self, event: &ArchiveEvent) -> ControlAction {
        match event {
            ArchiveEvent::Started(op_type) => self.on_started(*op_type),
            ArchiveEvent::EntryStarted(name) => self.on_entry_started(name),
            ArchiveEvent::Progress(info) => self.on_progress(info),
            ArchiveEvent::EntryFinished(name) => self.on_entry_finished(name),
            ArchiveEvent::Warning(msg) => self.on_warning(msg),
            ArchiveEvent::Finished => self.on_finished(),
        }
    }
}

/// A no-op handler that does nothing and always continues
pub struct NoOpHandler;

impl ArchiveHandler for NoOpHandler {
    fn on_event(&mut self, _event: &ArchiveEvent) -> ControlAction {
        ControlAction::Continue
    }
}
