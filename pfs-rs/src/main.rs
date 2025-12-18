use anyhow::Result;
use clap::CommandFactory;
use clap::{Parser, Subcommand};
use log::{error, info};
use pf8::{self, ArchiveHandler, ControlAction};
use pfs_rs::{determine_extract_output, determine_pack_output, util};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Unpack or pack Artemis pfs archive
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,
    /// Change to directory before performing operations
    #[arg(short = 'C', long = "directory", global = true)]
    directory: Option<PathBuf>,
    /// Force overwrite existing files
    #[arg(short = 'f', long = "force", global = true, default_value_t = false)]
    overwrite: bool,
    /// Quiet mode (no progress output)
    #[arg(short = 'q', long = "quiet", global = true, default_value_t = false)]
    quiet: bool,
    /// Verbose mode (show detailed information)
    #[arg(short = 'v', long = "verbose", global = true, default_value_t = false)]
    verbose: bool,
    /// Input file or dir use for drag-in
    #[arg(hide = true)]
    inputs: Vec<PathBuf>,
}

#[derive(Subcommand, Debug)]
enum Commands {
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

fn command_unpack_paths(
    paths: &[PathBuf],
    output: Option<&Path>,
    separate: bool,
    quiet: bool,
) -> Result<()> {
    for path in paths {
        let output_path = determine_extract_output(path, output, separate);
        fs::create_dir_all(&output_path)?;
        info!("Extracting {:?} to {:?}", path, output_path);

        let mut archive = pf8::Pf8Archive::open(path)?;

        // Use handler for progress tracking and statistics
        if quiet {
            let mut handler = pf8::callbacks::NoOpHandler;
            archive.extract_all_with_progress(&output_path, &mut handler)?;
        } else {
            let mut handler = ProgressHandler::new();
            archive.extract_all_with_progress(&output_path, &mut handler)?;

            // Use source pfs file size as total size
            let total_bytes = fs::metadata(path)?.len();
            handler.print_summary(total_bytes);
        }
    }
    Ok(())
}

fn command_pack(
    input: &Path,
    output: Option<&Path>,
    preserve_dir_name: bool,
    overwrite: bool,
    quiet: bool,
    no_smart_detect: bool,
) -> Result<()> {
    if !input.is_dir() {
        return Err(anyhow::anyhow!("Input must be a directory"));
    }

    let output_file = determine_pack_output(&[input.to_path_buf()], output, overwrite)?;
    info!("Creating archive {:?} from {:?}", output_file, input);

    // Smart detection: if directory contains system.ini, pack contents only
    // This handles classic PFS game directory structure automatically
    let has_system_ini = !no_smart_detect && util::has_system_ini(input);
    let should_preserve_dir = preserve_dir_name && !has_system_ini;

    if has_system_ini && preserve_dir_name {
        info!("Detected system.ini, packing directory contents only (classic PFS structure)");
    }

    let mut builder = pf8::Pf8Builder::new();

    if should_preserve_dir {
        // Pack directory itself (e.g., 'root/a' -> 'a/...')
        let dir_name = input
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("Cannot determine directory name"))?;
        builder.add_dir_as(input, dir_name)?;
    } else {
        // Pack directory contents only (e.g., 'root/' -> 'a/...', 'b/...')
        builder.add_dir(input)?;
    }

    if quiet {
        builder.write_to_file(&output_file)?;
    } else {
        let mut handler = ProgressHandler::new();
        builder.write_to_file_with_progress(&output_file, &mut handler)?;

        // Get archive file size
        let total_bytes = fs::metadata(&output_file)?.len();
        handler.print_summary(total_bytes);
    }

    Ok(())
}
/// Progress handler that collects statistics and prints progress
struct ProgressHandler {
    start_time: Instant,
    total_files: usize,
}

impl ProgressHandler {
    fn new() -> Self {
        Self {
            start_time: Instant::now(),
            total_files: 0,
        }
    }

    fn print_summary(&self, total_bytes: u64) {
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
    }
}

impl ArchiveHandler for ProgressHandler {
    fn on_entry_started(&mut self, name: &str) -> ControlAction {
        self.total_files += 1;
        info!("Processing: {}", name);
        ControlAction::Continue
    }
}

fn command_pack_multiple_inputs_with_flags(
    inpath_dirs: &[(PathBuf, bool)], // (path, preserve_dir_name)
    inpath_files: &[PathBuf],
    output: Option<&Path>,
    overwrite: bool,
    quiet: bool,
) -> Result<()> {
    // Combine all inputs for output determination
    let mut all_inputs: Vec<PathBuf> = inpath_dirs.iter().map(|(p, _)| p.clone()).collect();
    all_inputs.extend(inpath_files.iter().cloned());

    let output_file = determine_pack_output(&all_inputs, output, overwrite)?;
    info!("Creating archive {:?}", output_file);

    // Use new pf8 library API with builder
    let mut builder = pf8::Pf8Builder::new();

    // Add directories according to their flags
    for (dir, preserve_dir_name) in inpath_dirs {
        if *preserve_dir_name {
            // Preserve directory name (e.g., 'root/a' -> 'a/...')
            let dir_name = dir
                .file_name()
                .ok_or_else(|| anyhow::anyhow!("Cannot determine directory name for {:?}", dir))?;
            builder.add_dir_as(dir, dir_name)?;
        } else {
            // Pack contents only (e.g., 'root/a/' -> '...')
            builder.add_dir(dir)?;
        }
    }

    // Add individual files
    for file in inpath_files {
        builder.add_file(file)?;
    }

    if quiet {
        builder.write_to_file(&output_file)?;
    } else {
        let mut handler = ProgressHandler::new();
        builder.write_to_file_with_progress(&output_file, &mut handler)?;

        // Get archive file size
        let total_bytes = fs::metadata(&output_file)?.len();
        handler.print_summary(total_bytes);
    }

    Ok(())
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    if let Err(e) = run() {
        error!("Fatal error: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Args::parse();

    // Change directory if specified
    if let Some(dir) = &cli.directory {
        std::env::set_current_dir(dir)
            .map_err(|e| anyhow::anyhow!("Failed to change directory to {:?}: {}", dir, e))?;
        info!("Changed working directory to {:?}", dir);
    }

    let overwrite = cli.overwrite;
    let quiet = cli.quiet;
    let verbose = cli.verbose;

    // Set log level based on verbose/quiet flags
    if verbose && !quiet {
        log::set_max_level(log::LevelFilter::Debug);
    }

    match &cli.command {
        Some(command) => match command {
            Commands::Extract {
                input,
                output,
                separate,
                strip_components,
            } => {
                let files = util::glob_expand(input)?;
                if let Some(_strips) = strip_components {
                    log::warn!("--strip-components is not yet implemented");
                }
                command_unpack_paths(&files, output.as_deref(), *separate, quiet)?;
            }
            Commands::Create {
                inputs,
                output,
                no_smart_detect,
            } => {
                // Parse inputs with rsync-style trailing slash semantics
                // input_str, path, preserve_dir_name
                let mut parsed_inputs: Vec<(String, PathBuf, bool)> = Vec::new();

                for input_str in inputs {
                    let path = PathBuf::from(&input_str);
                    if !path.exists() {
                        return Err(anyhow::anyhow!(
                            "Input path does not exist: {:?}",
                            input_str
                        ));
                    }

                    if path.is_dir() {
                        // Check if original string ends with / or /.
                        let has_trailing_slash =
                            input_str.ends_with('/') || input_str.ends_with("/.");
                        // trailing slash means: pack contents only (don't preserve dir name)
                        // no trailing slash means: pack dir itself (preserve dir name)
                        let preserve_dir_name = !has_trailing_slash;
                        parsed_inputs.push((input_str.clone(), path, preserve_dir_name));
                    } else {
                        // For files, always add as-is
                        parsed_inputs.push((input_str.clone(), path, false));
                    }
                }

                if parsed_inputs.is_empty() {
                    return Err(anyhow::anyhow!("No valid inputs provided"));
                }

                // If only one input, use simple pack
                if parsed_inputs.len() == 1 {
                    let (_, path, preserve_dir_name) = &parsed_inputs[0];
                    if path.is_dir() {
                        command_pack(
                            path,
                            output.as_deref(),
                            *preserve_dir_name,
                            overwrite,
                            quiet,
                            *no_smart_detect,
                        )?;
                    } else {
                        // Single file - use multiple inputs handler
                        command_pack_multiple_inputs_with_flags(
                            &[],
                            std::slice::from_ref(path),
                            output.as_deref(),
                            overwrite,
                            quiet,
                        )?;
                    }
                } else {
                    // Multiple inputs - separate dirs and files
                    let mut dirs_with_flags: Vec<(PathBuf, bool)> = Vec::new();
                    let mut files = Vec::new();

                    for (_, path, preserve_dir_name) in parsed_inputs {
                        if path.is_dir() {
                            dirs_with_flags.push((path, preserve_dir_name));
                        } else {
                            files.push(path);
                        }
                    }

                    command_pack_multiple_inputs_with_flags(
                        &dirs_with_flags,
                        &files,
                        output.as_deref(),
                        overwrite,
                        quiet,
                    )?;
                }
            }
            Commands::List { input, long } => {
                #[cfg(feature = "display")]
                {
                    if *long {
                        pf8::display::list_archive(input)?;
                    } else {
                        // Simple list
                        let archive = pf8::Pf8Archive::open(input)?;
                        for entry in archive.entries() {
                            println!("{}", entry.path().display());
                        }
                    }
                }

                #[cfg(not(feature = "display"))]
                {
                    let archive = pf8::Pf8Archive::open(input)?;
                    if *long {
                        println!("{}", input.display());
                        println!();
                        for entry in archive.entries() {
                            println!("{}: {} bytes", entry.path().display(), entry.size());
                        }
                    } else {
                        for entry in archive.entries() {
                            println!("{}", entry.path().display());
                        }
                    }
                }
            }
        },
        None => {
            if !cli.inputs.is_empty() {
                match util::process_cli_inputs(cli.inputs) {
                    Ok(result) => {
                        match result {
                            util::InputType::PfsFiles(pfs_files) => {
                                // Extract operation - use auto-detect
                                command_unpack_paths(&pfs_files, None, true, quiet)?;
                            }
                            util::InputType::PackFiles { dirs, files } => {
                                // Pack operation - use auto-detect
                                // Behavior:
                                // - Multiple directories: always preserve directory names
                                // - Single directory with system.ini: pack contents only (game structure)
                                // - Single directory without system.ini: preserve directory name
                                let is_single_dir = dirs.len() == 1 && files.is_empty();

                                let dirs_with_flags: Vec<(PathBuf, bool)> = dirs
                                    .into_iter()
                                    .map(|d| {
                                        if is_single_dir {
                                            // Single directory: check for system.ini
                                            let has_system_ini = util::has_system_ini(&d);
                                            if has_system_ini {
                                                info!("Detected system.ini in {:?}, packing contents only", d);
                                                (d, false) // Don't preserve dir name
                                            } else {
                                                (d, true) // Preserve dir name
                                            }
                                        } else {
                                            // Multiple directories: always preserve names
                                            (d, true)
                                        }
                                    })
                                    .collect();
                                command_pack_multiple_inputs_with_flags(
                                    &dirs_with_flags,
                                    &files,
                                    None,
                                    overwrite,
                                    quiet,
                                )?;
                            }
                        }
                    }
                    Err(e) => {
                        error!("Error processing inputs: {e}");
                        std::process::exit(1);
                    }
                }
            } else {
                let mut cmd = Args::command();
                cmd.print_help()?;
            }
        }
    }
    Ok(())
}
