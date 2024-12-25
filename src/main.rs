use anyhow::Result;
use clap::{Parser, Subcommand};
use log::{debug, info};
use pfs_rs::pf8;
use pfs_rs::util;
use std::env;
use std::fs;
use std::path::PathBuf;

/// Unpack or pack Artemis pfs archive
#[derive(Parser, Debug)]
#[command(version, about,long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Unpack a Artemis pfs archive.
    ///
    /// Will also unpack related pfs files.
    Unpack {
        /// Input file, ending in .pfs
        input: PathBuf,
        /// Output directory
        output: PathBuf,
        /// Unpack single file rather than all related files
        #[arg(short, long, default_value_t = false)]
        split_output: bool,
    },
    /// Pack a directory into a Artemis pfs archive
    Pack { input: PathBuf, output: PathBuf },
}

fn main() -> Result<()> {
    env::set_var("RUST_LOG", "info");
    env_logger::init();

    let unencrypted_filter: Vec<&str> = vec!["mp4", "flv"];
    let cli = Args::parse();
    match &cli.command {
        Commands::Unpack {
            input,
            output,
            split_output,
        } => {
            let pfs = util::find_pfs_files(input)?;
            let pfs_count = pfs.len();

            let output_path = if *split_output {
                let unpack_name =
                    format!("{}.unpack", input.file_name().unwrap().to_str().unwrap());
                input.with_file_name(unpack_name)
            } else {
                output.join(util::get_pfs_basename(pfs[0].as_path())?)
            };
            fs::create_dir_all(&output_path)?;

            for i in pfs {
                info!("Unpacking {:?}", i);
                pf8::unpack_pf8(&i, &output_path, unencrypted_filter.clone(), None)?;
                debug!("Unpacked {:?}", i);
            }

            info!("Completed unpacking {} pfs files", pfs_count);
        }
        Commands::Pack { input, output } => {
            if !input.is_dir() {
                panic!("Input must be a directory");
            }
            let output_file = if output.is_dir() {
                let pack_name = format!("{}.pfs", input.file_name().unwrap().to_str().unwrap());
                &output.join(pack_name)
            } else {
                output
            };
            info!("Packing {:?} to {:?}", input, output_file);
            pf8::pack_pf8(
                &PathBuf::from(input),
                &PathBuf::from(output_file),
                unencrypted_filter,
            )?;
            info!("Completed packing");
        }
    }
    Ok(())
}
