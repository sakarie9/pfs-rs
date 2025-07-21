use anyhow::Result;
use clap::CommandFactory;
use clap::{Parser, Subcommand};
use log::info;
use pfs_rs::pf8;
use pfs_rs::util;
use std::fs;
use std::path::{Path, PathBuf};

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

fn command_unpack(
    input: &Vec<PathBuf>,
    output: &Path,
    split_output: bool,
    filters: &[&str],
) -> Result<()> {
    let output_path = if split_output {
        let unpack_name = format!("{}.unpack", input[0].file_name().unwrap().to_str().unwrap());
        input[0].with_file_name(unpack_name)
    } else {
        output.join(util::get_pfs_basename(input[0].as_path())?)
    };
    fs::create_dir_all(&output_path)?;

    for i in input {
        info!("Unpacking {:?}", i);
        pf8::unpack_pf8(i, &output_path, filters.to_vec(), None)?;
        info!("Unpacked {:?} to {}", i, output_path.display());
    }

    info!("Completed unpacking {} pfs files", input.len());
    Ok(())
}
fn command_pack(input: &Path, output: &Path, filters: &[&str], overwrite: bool) -> Result<()> {
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
    pf8::pack_pf8(&PathBuf::from(input), &PathBuf::from(output_file), filters)?;
    info!("Completed packing");
    Ok(())
}
fn command_pack_multiple_inputs(
    inpath_dirs: &[PathBuf],
    inpath_files: &[PathBuf],
    output: &Path,
    filters: &[&str],
) -> Result<()> {
    info!("Packing to {:?}", output);
    pf8::pack_pf8_multi_input(inpath_dirs, inpath_files, output, filters)?;
    info!("Completed packing");
    Ok(())
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let unencrypted_filter: Vec<&str> = vec!["mp4", "flv"];
    let cli = Args::parse();
    let overwrite = cli.overwrite;
    match &cli.command {
        Some(command) => match command {
            Commands::Unpack {
                input,
                output,
                split_output,
            } => {
                let files = util::glob_expand(input)?;
                command_unpack(&files, output, *split_output, &unencrypted_filter)?;
            }
            Commands::Pack { input, output } => {
                command_pack(input, output, &unencrypted_filter, overwrite)?;
            }
            Commands::List { input } => {
                pf8::list_pf8(input)?;
            }
        },
        None => {
            if !cli.inputs.is_empty() {
                match util::process_cli_inputs(cli.inputs) {
                    Ok(result) => {
                        match result.input_type {
                            util::InputType::PfsFiles(pfs_files) => {
                                // 解包操作
                                let output = result.suggested_output.ok_or_else(|| {
                                    anyhow::anyhow!("Cannot determine output path for unpacking")
                                })?;
                                command_unpack(&pfs_files, &output, true, &unencrypted_filter)?;
                            }
                            util::InputType::PackFiles { dirs, files } => {
                                // 打包操作
                                let suggested_output =
                                    result.suggested_output.ok_or_else(|| {
                                        anyhow::anyhow!("Cannot determine output path for packing")
                                    })?;
                                let final_output =
                                    util::get_final_output_path(suggested_output, overwrite)?;
                                command_pack_multiple_inputs(
                                    &dirs,
                                    &files,
                                    &final_output,
                                    &unencrypted_filter,
                                )?;
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
