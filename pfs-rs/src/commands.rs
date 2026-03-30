use anyhow::Result;
use log::info;
use std::fs;
use std::path::{Path, PathBuf};

use crate::progress::ProgressHandler;
use crate::util;
use crate::util::{determine_extract_output, determine_pack_output};

/// Extract PFS archives to the specified output directory.
pub fn command_extract(
    paths: &[PathBuf],
    output: Option<&Path>,
    separate: bool,
    quiet: bool,
) -> Result<()> {
    for path in paths {
        let output_path = determine_extract_output(path, output, separate);
        fs::create_dir_all(&output_path)?;
        if !quiet {
            info!("Extracting {:?} to {:?}", path, output_path);
        }

        let mut archive = pf8::Pf8Archive::open(path)?;

        if quiet {
            let mut handler = pf8::callbacks::NoOpHandler;
            archive.extract_all_with_progress(&output_path, &mut handler)?;
        } else {
            let mut handler = ProgressHandler::new();
            archive.extract_all_with_progress(&output_path, &mut handler)?;

            let total_bytes = fs::metadata(path)?.len();
            handler.print_summary(total_bytes, &output_path);
        }
    }
    Ok(())
}

/// Create a PFS archive from a single directory.
pub fn command_create(
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
    if !quiet {
        info!("Creating archive {:?} from {:?}", output_file, input);
    }

    // Smart detection: if directory contains system.ini, pack contents only
    // This handles classic PFS game directory structure automatically
    let has_system_ini = !no_smart_detect && util::has_system_ini(input);
    let should_preserve_dir = preserve_dir_name && !has_system_ini;

    if has_system_ini && preserve_dir_name && !quiet {
        info!("Detected system.ini, packing directory contents only (classic PFS structure)");
    }

    let mut builder = pf8::Pf8Builder::new();

    if should_preserve_dir {
        let dir_name = input
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("Cannot determine directory name"))?;
        builder.add_dir_as(input, dir_name)?;
    } else {
        builder.add_dir(input)?;
    }

    write_archive(builder, &output_file, quiet)?;
    Ok(())
}

/// Create a PFS archive from multiple directories and files.
pub fn command_create_multiple(
    inpath_dirs: &[(PathBuf, bool)], // (path, preserve_dir_name)
    inpath_files: &[PathBuf],
    output: Option<&Path>,
    overwrite: bool,
    quiet: bool,
) -> Result<()> {
    let mut all_inputs: Vec<PathBuf> = inpath_dirs.iter().map(|(p, _)| p.clone()).collect();
    all_inputs.extend(inpath_files.iter().cloned());

    let output_file = determine_pack_output(&all_inputs, output, overwrite)?;
    info!("Creating archive {:?}", output_file);

    let mut builder = pf8::Pf8Builder::new();

    for (dir, preserve_dir_name) in inpath_dirs {
        if *preserve_dir_name {
            let dir_name = dir
                .file_name()
                .ok_or_else(|| anyhow::anyhow!("Cannot determine directory name for {:?}", dir))?;
            builder.add_dir_as(dir, dir_name)?;
        } else {
            builder.add_dir(dir)?;
        }
    }

    for file in inpath_files {
        builder.add_file(file)?;
    }

    write_archive(builder, &output_file, quiet)?;
    Ok(())
}

/// List contents of a PFS archive.
pub fn command_list(input: &Path, long: bool) -> Result<()> {
    #[cfg(feature = "display")]
    {
        if long {
            pf8::display::list_archive(input)?;
        } else {
            let archive = pf8::Pf8Archive::open(input)?;
            for entry in archive.entries() {
                println!("{}", entry.path().display());
            }
        }
    }

    #[cfg(not(feature = "display"))]
    {
        let archive = pf8::Pf8Archive::open(input)?;
        if long {
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

    Ok(())
}

/// Write a builder to an archive file, with optional progress tracking.
fn write_archive(builder: pf8::Pf8Builder, output_file: &Path, quiet: bool) -> Result<()> {
    if quiet {
        builder.write_to_file(output_file)?;
    } else {
        let mut handler = ProgressHandler::new();
        builder.write_to_file_with_progress(output_file, &mut handler)?;

        let total_bytes = fs::metadata(output_file)?.len();
        handler.print_summary(total_bytes, output_file);
    }
    Ok(())
}
