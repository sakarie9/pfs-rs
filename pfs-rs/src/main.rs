use anyhow::Result;
use clap::CommandFactory;
use clap::{Parser, Subcommand};
use log::{error, info};
use pf8::{self, ArchiveHandler, ControlAction};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

mod util;

/// Unpack or pack Artemis pfs archive
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,
    /// Force overwrite existing files
    #[arg(short = 'f', long = "overwrite", default_value_t = false)]
    overwrite: bool,
    /// Quiet mode (no progress output)
    #[arg(short = 'q', long = "quiet", default_value_t = false)]
    quiet: bool,
    /// Input file or dir use for drag-in
    #[arg(hide = true)]
    inputs: Vec<PathBuf>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Unpack a Artemis pfs archive.
    ///
    /// Will also unpack related pfs files.
    #[command(alias = "u")]
    Unpack {
        /// Input pfs file, can be a glob pattern
        input: String,
        /// Output directory
        output: PathBuf,
        /// Unpack single file rather than all related files
        #[arg(short, long, default_value_t = false)]
        split_output: bool,
    },
    /// Pack a directory into a Artemis pfs archive
    #[command(alias = "p")]
    Pack {
        /// Input directory
        input: PathBuf,
        /// Output pfs file
        output: PathBuf,
    },
    /// List contents of a Artemis pfs archive
    #[command(alias = "ls")]
    List {
        /// Input pfs file
        input: PathBuf,
    },
}

fn command_unpack_paths(
    paths: &[PathBuf],
    output: &Path,
    split_output: bool,
    quiet: bool,
) -> Result<()> {
    for path in paths {
        let output_path = if split_output {
            let filename = path.file_stem().unwrap();
            output.join(filename)
        } else {
            output.to_path_buf()
        };
        fs::create_dir_all(&output_path)?;
        info!("Unpacking {path:?} to {output_path:?}");

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
fn command_pack(input: &Path, output: &Path, overwrite: bool, quiet: bool) -> Result<()> {
    if !input.is_dir() {
        panic!("Input must be a directory");
    }
    let output_file = if output.is_dir() {
        if overwrite {
            let pack_name = format!("{}.pfs", input.file_name().unwrap().to_str().unwrap());
            &output.join(pack_name)
        } else {
            &util::try_get_next_nonexist_pfs(
                output,
                util::get_pfs_basename(input).unwrap().as_str(),
            )?
        }
    } else {
        output
    };
    info!("Packing {input:?} to {output_file:?}");

    if quiet {
        pf8::create_from_dir(input, output_file)?;
    } else {
        let mut handler = ProgressHandler::new();
        pf8::create_from_dir_with_progress(input, output_file, &mut handler)?;

        // Get archive file size
        let total_bytes = fs::metadata(output_file)?.len();
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

fn command_pack_multiple_inputs(
    inpath_dirs: &[PathBuf],
    inpath_files: &[PathBuf],
    output: &Path,
    quiet: bool,
) -> Result<()> {
    info!("Packing to {output:?}");

    // Use new pf8 library API with builder
    let mut builder = pf8::Pf8Builder::new();

    // Add directories
    for dir in inpath_dirs {
        builder.add_dir(dir)?;
    }

    // Add individual files
    for file in inpath_files {
        builder.add_file(file)?;
    }

    if quiet {
        builder.write_to_file(output)?;
    } else {
        let mut handler = ProgressHandler::new();
        builder.write_to_file_with_progress(output, &mut handler)?;

        // Get archive file size
        let total_bytes = fs::metadata(output)?.len();
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
    let overwrite = cli.overwrite;
    let quiet = cli.quiet;
    match &cli.command {
        Some(command) => match command {
            Commands::Unpack {
                input,
                output,
                split_output,
            } => {
                let files = util::glob_expand(input)?;
                command_unpack_paths(&files, output, *split_output, quiet)?;
            }
            Commands::Pack { input, output } => {
                command_pack(input, output, overwrite, quiet)?;
            }
            Commands::List { input } => {
                #[cfg(feature = "display")]
                pf8::display::list_archive(input)?;

                #[cfg(not(feature = "display"))]
                {
                    let archive = pf8::Pf8Archive::open(input)?;
                    println!("{}", input.display());
                    println!();
                    for entry in archive.entries()? {
                        println!("{}: {} bytes", entry.path().display(), entry.size());
                    }
                }
            }
        },
        None => {
            if !cli.inputs.is_empty() {
                match util::process_cli_inputs(cli.inputs) {
                    Ok(result) => {
                        match result.input_type {
                            util::InputType::PfsFiles(pfs_files) => {
                                // 解包操作 - 使用默认设置
                                let output = result.suggested_output.ok_or_else(|| {
                                    anyhow::anyhow!("Cannot determine output path for unpacking")
                                })?;
                                command_unpack_paths(&pfs_files, &output, true, quiet)?;
                            }
                            util::InputType::PackFiles { dirs, files } => {
                                // 打包操作
                                let suggested_output =
                                    result.suggested_output.ok_or_else(|| {
                                        anyhow::anyhow!("Cannot determine output path for packing")
                                    })?;
                                let final_output =
                                    util::get_final_output_path(suggested_output, overwrite)?;
                                command_pack_multiple_inputs(&dirs, &files, &final_output, quiet)?;
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
