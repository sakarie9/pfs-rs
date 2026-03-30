use anyhow::Result;
use clap::{CommandFactory, Parser};
use log::{error, info};
use pfs_rs::cli::{Args, Commands};
use pfs_rs::commands;
use pfs_rs::util;
use std::path::PathBuf;

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp(None)
        .init();

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
        Some(Commands::Extract {
            input,
            output,
            separate,
            strip_components,
        }) => {
            let files = util::glob_expand(input)?;
            if let Some(_strips) = strip_components {
                log::warn!("--strip-components is not yet implemented");
            }
            commands::command_extract(&files, output.as_deref(), *separate, quiet)?;
        }
        Some(Commands::Create {
            inputs,
            output,
            no_smart_detect,
        }) => {
            handle_create(
                inputs,
                output.as_deref(),
                overwrite,
                quiet,
                *no_smart_detect,
            )?;
        }
        Some(Commands::List { input, long }) => {
            commands::command_list(input, *long)?;
        }
        None => {
            handle_drag_in(cli.inputs, overwrite, quiet)?;
        }
    }
    Ok(())
}

/// Handle the `create` subcommand with rsync-style trailing slash semantics.
fn handle_create(
    inputs: &[String],
    output: Option<&std::path::Path>,
    overwrite: bool,
    quiet: bool,
    no_smart_detect: bool,
) -> Result<()> {
    // Parse inputs: (raw_string, resolved_path, preserve_dir_name)
    let mut parsed_inputs: Vec<(PathBuf, bool)> = Vec::new();

    for input_str in inputs {
        let path = PathBuf::from(input_str);
        if !path.exists() {
            return Err(anyhow::anyhow!(
                "Input path does not exist: {:?}",
                input_str
            ));
        }

        if path.is_dir() {
            let has_trailing_slash = input_str.ends_with('/') || input_str.ends_with("/.");
            // trailing slash → pack contents only; no trailing slash → preserve dir name
            parsed_inputs.push((path, !has_trailing_slash));
        } else {
            parsed_inputs.push((path, false));
        }
    }

    if parsed_inputs.is_empty() {
        return Err(anyhow::anyhow!("No valid inputs provided"));
    }

    // Single directory input: use simple pack with smart detection
    if parsed_inputs.len() == 1 && parsed_inputs[0].0.is_dir() {
        let (path, preserve_dir_name) = &parsed_inputs[0];
        return commands::command_create(
            path,
            output,
            *preserve_dir_name,
            overwrite,
            quiet,
            no_smart_detect,
        );
    }

    // Multiple inputs or single file: separate dirs and files
    let mut dirs_with_flags: Vec<(PathBuf, bool)> = Vec::new();
    let mut files = Vec::new();

    for (path, preserve_dir_name) in parsed_inputs {
        if path.is_dir() {
            dirs_with_flags.push((path, preserve_dir_name));
        } else {
            files.push(path);
        }
    }

    commands::command_create_multiple(&dirs_with_flags, &files, output, overwrite, quiet)
}

/// Handle drag-in mode (no subcommand): auto-detect extract or pack.
fn handle_drag_in(inputs: Vec<PathBuf>, overwrite: bool, quiet: bool) -> Result<()> {
    if inputs.is_empty() {
        let mut cmd = Args::command();
        cmd.print_help()?;
        return Ok(());
    }

    match util::process_cli_inputs(inputs) {
        Ok(util::InputType::PfsFiles(pfs_files)) => {
            commands::command_extract(&pfs_files, None, true, quiet)?;
        }
        Ok(util::InputType::PackFiles { dirs, files }) => {
            // Single directory with system.ini → pack contents only (game structure)
            let is_single_dir = dirs.len() == 1 && files.is_empty();

            let dirs_with_flags: Vec<(PathBuf, bool)> = dirs
                .into_iter()
                .map(|d| {
                    if is_single_dir && util::has_system_ini(&d) {
                        info!("Detected system.ini in {:?}, packing contents only", d);
                        (d, false)
                    } else {
                        (d, true)
                    }
                })
                .collect();

            commands::command_create_multiple(&dirs_with_flags, &files, None, overwrite, quiet)?;
        }
        Err(e) => {
            error!("Error processing inputs: {e}");
            std::process::exit(1);
        }
    }
    Ok(())
}
