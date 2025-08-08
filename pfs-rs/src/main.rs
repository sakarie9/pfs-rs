use anyhow::Result;
use clap::CommandFactory;
use clap::{Parser, Subcommand};
use log::info;
use pf8::{self};
use std::fs;
use std::path::{Path, PathBuf};

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
        /// Buffer size for memory optimization (in KiB)
        #[arg(long, default_value_t = 4096)]
        buffer_size: usize,
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
    buffer_size: usize,
    filters: Option<&[&str]>,
) -> Result<()> {
    let buffer_bytes = buffer_size * 1024; // Convert KiB to bytes

    for path in paths {
        let output_path = if split_output {
            let filename = path.file_stem().unwrap();
            output.join(filename)
        } else {
            output.to_path_buf()
        };
        fs::create_dir_all(&output_path)?;
        info!(
            "Unpacking {:?} to {:?} with {}KiB buffer",
            path, output_path, buffer_size
        );

        let mut archive = if let Some(filters) = filters {
            pf8::Pf8Archive::open_with_patterns(path, filters)?
        } else {
            pf8::Pf8Archive::open(path)?
        };

        // Use memory-optimized extraction with specified buffer
        archive.extract_all_with_buffer_size(&output_path, buffer_bytes)?;

        info!("Completed unpacking");
    }
    Ok(())
}
fn command_pack(
    input: &Path,
    output: &Path,
    filters: Option<&[&str]>,
    overwrite: bool,
) -> Result<()> {
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
    info!("Packing {:?} to {:?}", input, output_file);

    match filters {
        Some(filters) => pf8::create_from_dir_with_patterns(input, output_file, filters),
        None => pf8::create_from_dir(input, output_file),
    }?;

    info!("Completed packing");
    Ok(())
}
fn command_pack_multiple_inputs(
    inpath_dirs: &[PathBuf],
    inpath_files: &[PathBuf],
    output: &Path,
    filters: Option<&[&str]>,
) -> Result<()> {
    info!("Packing to {:?}", output);

    // Use new pf8 library API with builder
    let mut builder = pf8::Pf8Builder::new();

    if let Some(filters) = filters {
        builder.unencrypted_patterns(filters);
    }

    // Add directories
    for dir in inpath_dirs {
        builder.add_dir(dir)?;
    }

    // Add individual files
    for file in inpath_files {
        builder.add_file(file)?;
    }

    builder.write_to_file(output)?;

    info!("Completed packing");
    Ok(())
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let cli = Args::parse();
    let overwrite = cli.overwrite;
    match &cli.command {
        Some(command) => match command {
            Commands::Unpack {
                input,
                output,
                split_output,
                buffer_size,
            } => {
                let files = util::glob_expand(input)?;
                command_unpack_paths(&files, output, *split_output, *buffer_size, None)?;
            }
            Commands::Pack { input, output } => {
                command_pack(input, output, None, overwrite)?;
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
                                command_unpack_paths(&pfs_files, &output, true, 4096, None)?;
                            }
                            util::InputType::PackFiles { dirs, files } => {
                                // 打包操作
                                let suggested_output =
                                    result.suggested_output.ok_or_else(|| {
                                        anyhow::anyhow!("Cannot determine output path for packing")
                                    })?;
                                let final_output =
                                    util::get_final_output_path(suggested_output, overwrite)?;
                                command_pack_multiple_inputs(&dirs, &files, &final_output, None)?;
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error processing inputs: {}", e);
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
