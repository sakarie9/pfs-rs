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
    Unpack {
        /// Input file, ending in .pfs, can be a glob pattern
        input: String,
        /// Output directory
        output: PathBuf,
        /// Unpack single file rather than all related files
        #[arg(short, long, default_value_t = false)]
        split_output: bool,
    },
    /// Pack a directory into a Artemis pfs archive
    Pack {
        /// Input directory
        input: PathBuf,
        /// Output file, ending in .pfs
        output: PathBuf,
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
        },
        None => {
            if !cli.inputs.is_empty() {
                let mut input_pfs = Vec::new();
                let mut input_dirs = Vec::new();
                let mut input_files = Vec::new();
                for input in cli.inputs {
                    info!("Input: {:?}", input);
                    if input.is_dir() {
                        input_dirs.push(input);
                    } else if util::is_file_pf8_from_filename(input.as_path()) {
                        input_pfs.push(input);
                    } else if input.is_file() {
                        input_files.push(input);
                    } else {
                        panic!("Invalid input");
                    }
                }
                let is_empty_pfs = input_pfs.is_empty();
                let is_empty_pack = input_dirs.is_empty() && input_files.is_empty();
                if is_empty_pfs && is_empty_pack {
                    panic!("Invalid input");
                } else if !is_empty_pfs && !is_empty_pack {
                    panic!("Mixing input pfses and files to pack");
                } else if is_empty_pack {
                    // unpack pfs
                    let output = util::get_pfs_basepath(input_pfs[0].as_path())?;
                    command_unpack(&input_pfs, output.as_path(), true, &unencrypted_filter)?;
                } else {
                    // pack
                    let base_dir = if input_dirs.is_empty() {
                        input_files[0].parent().unwrap()
                    } else {
                        input_dirs[0].parent().unwrap()
                    };

                    let output = if overwrite {
                        base_dir.join("root.pfs")
                    } else {
                        util::try_get_next_nonexist_pfs(base_dir, "root")?
                    };
                    command_pack_multiple_inputs(
                        &input_dirs,
                        &input_files,
                        output.as_path(),
                        &unencrypted_filter,
                    )?;
                }
            } else {
                let mut cmd = Args::command();
                cmd.print_help()?;
            }
        }
    }
    Ok(())
}
