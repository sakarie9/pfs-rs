use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Unpack or pack Artemis pfs archive
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Commands>,
    /// Change to directory before performing operations
    #[arg(short = 'C', long = "directory", global = true)]
    pub directory: Option<PathBuf>,
    /// Force overwrite existing files
    #[arg(short = 'f', long = "force", global = true, default_value_t = false)]
    pub overwrite: bool,
    /// Quiet mode (no progress output)
    #[arg(short = 'q', long = "quiet", global = true, default_value_t = false)]
    pub quiet: bool,
    /// Verbose mode (show detailed information)
    #[arg(short = 'v', long = "verbose", global = true, default_value_t = false)]
    pub verbose: bool,
    /// Input file or dir use for drag-in
    #[arg(hide = true)]
    pub inputs: Vec<PathBuf>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Extract files from pfs archive(s).
    ///
    /// If output is not specified, extracts to current directory or creates
    /// a directory based on the archive name.
    #[command(visible_alias = "x", alias = "unpack", alias = "u")]
    Extract {
        /// Input pfs file(s), can be a glob pattern
        input: String,
        /// Output directory (optional, default: auto-detect)
        output: Option<PathBuf>,
        /// Extract each archive to separate directories
        #[arg(short = 's', long, default_value_t = false)]
        separate: bool,
        /// Strip NUMBER leading components from file names on extraction
        #[arg(long, value_name = "NUMBER")]
        strip_components: Option<usize>,
    },
    /// Create pfs archive from files/directories
    ///
    /// If output is not specified, creates archive with name based on input.
    /// Supports rsync-style trailing slash semantics:
    /// - 'dir/' packs contents of dir (a/1.file, b/2.file)
    /// - 'dir' packs dir itself (dir/a/1.file, dir/b/2.file)
    #[command(visible_alias = "c", alias = "pack", alias = "p")]
    Create {
        /// Input file(s) or directory (supports trailing / for rsync-style behavior)
        #[arg(required = true)]
        inputs: Vec<String>,
        /// Output pfs file (optional, default: root.pfs)
        #[arg(short = 'o', long = "output")]
        output: Option<PathBuf>,
        /// Disable smart detection (e.g., system.ini auto-pathstrip)
        #[arg(long, default_value_t = false)]
        no_smart_detect: bool,
    },
    /// List contents of pfs archive
    #[command(visible_alias = "l", alias = "ls")]
    List {
        /// Input pfs file
        input: PathBuf,
        /// Show detailed information
        #[arg(short = 'l', long, default_value_t = false)]
        long: bool,
    },
}
