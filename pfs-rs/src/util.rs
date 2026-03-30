use anyhow::{Result, anyhow};
use std::path::{Path, PathBuf};

// --- PFS path helpers ---

/// Returns true if the path looks like a PFS archive (contains ".pfs" in the filename).
pub fn is_file_pf8_from_filename(path: &Path) -> bool {
    path.file_name()
        .and_then(|s| s.to_str())
        .map(|name| name.contains(".pfs"))
        .unwrap_or(false)
}

/// Returns the stem before ".pfs" in the filename (e.g. `game.pfs.000` → `"game"`).
/// Falls back to the full filename if ".pfs" is not present.
pub fn get_pfs_basename(input: &Path) -> Result<String> {
    let name = input
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("Failed to get file name"))?;

    if let Some(pos) = name.find(".pfs") {
        Ok(name[..pos].to_string())
    } else {
        Ok(name.to_string())
    }
}

/// Returns the parent path joined with the stem before ".pfs"
/// (e.g. `/data/game.pfs.000` → `/data/game`).
pub fn get_pfs_basepath(input: &Path) -> Result<PathBuf> {
    let name = input
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("Failed to get file name"))?;

    let pos = name
        .find(".pfs")
        .ok_or_else(|| anyhow!("Invalid file name"))?;
    let parent = input.parent().unwrap_or(Path::new(""));
    Ok(parent.join(&name[..pos]))
}

/// Returns the first non-existing path of the form `<dir>/<base>.pfs`,
/// then `<dir>/<base>.pfs.000`, `.001`, … .
pub fn try_get_next_nonexist_pfs(dir: &Path, base: &str) -> Result<PathBuf> {
    let candidate = dir.join(format!("{base}.pfs"));
    if !candidate.exists() {
        return Ok(candidate);
    }
    for i in 0.. {
        let candidate = dir.join(format!("{base}.pfs.{i:03}"));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    unreachable!()
}

// --- Output path resolution ---

/// Resolve the extraction output directory from CLI arguments.
pub fn determine_extract_output(
    input: &Path,
    specified_output: Option<&Path>,
    separate: bool,
) -> PathBuf {
    match specified_output {
        Some(output) if separate => {
            // Separate mode: create a subdirectory named after the archive stem.
            let stem = input
                .file_stem()
                .unwrap_or_else(|| input.file_name().unwrap());
            output.join(stem)
        }
        Some(output) => output.to_path_buf(),
        None => get_pfs_basepath(input).unwrap_or_else(|_| input.with_extension("")),
    }
}

/// Resolve the pack output file path from CLI arguments.
pub fn determine_pack_output(
    _inputs: &[PathBuf],
    specified_output: Option<&Path>,
    overwrite: bool,
) -> Result<PathBuf> {
    match specified_output {
        Some(output) if output.is_dir() => {
            if overwrite {
                Ok(output.join("root.pfs"))
            } else {
                try_get_next_nonexist_pfs(output, "root")
            }
        }
        Some(output) => Ok(output.to_path_buf()),
        None => {
            let cwd = std::env::current_dir()?;
            if overwrite {
                Ok(cwd.join("root.pfs"))
            } else {
                try_get_next_nonexist_pfs(&cwd, "root")
            }
        }
    }
}

// --- Input classification (drag-in / no-subcommand mode) ---

/// Describes what kind of inputs were passed on the command line.
#[derive(Debug, Clone)]
pub enum InputType {
    /// One or more PFS archives to extract.
    PfsFiles(Vec<PathBuf>),
    /// Directories and/or loose files to pack into an archive.
    PackFiles {
        dirs: Vec<PathBuf>,
        files: Vec<PathBuf>,
    },
}

/// Classify a mixed list of CLI inputs into either an extract or a pack operation.
/// Returns an error if PFS archives and pack inputs are mixed together.
pub fn process_cli_inputs(inputs: Vec<PathBuf>) -> Result<InputType> {
    if inputs.is_empty() {
        return Err(anyhow!("No input provided"));
    }

    let mut pfs_files = Vec::new();
    let mut directories = Vec::new();
    let mut regular_files = Vec::new();

    for input in inputs {
        if !input.exists() {
            return Err(anyhow!("Input path does not exist: {:?}", input));
        }
        if input.is_dir() {
            directories.push(input);
        } else if is_file_pf8_from_filename(&input) {
            pfs_files.push(input);
        } else if input.is_file() {
            regular_files.push(input);
        } else {
            return Err(anyhow!("Invalid input type: {:?}", input));
        }
    }

    let has_pfs = !pfs_files.is_empty();
    let has_pack = !directories.is_empty() || !regular_files.is_empty();

    match (has_pfs, has_pack) {
        (true, false) => Ok(InputType::PfsFiles(pfs_files)),
        (false, true) => Ok(InputType::PackFiles {
            dirs: directories,
            files: regular_files,
        }),
        (true, true) => Err(anyhow!(
            "Cannot mix PFS files and pack inputs (directories/files) in the same operation"
        )),
        (false, false) => Err(anyhow!("No valid input found")),
    }
}

// --- Miscellaneous ---

/// Returns true if the directory contains a `system.ini` file (classic PFS game structure).
pub fn has_system_ini(dir: &Path) -> bool {
    dir.join("system.ini").exists()
}

/// Expand a glob pattern to a list of paths, returning an error if nothing matches.
pub fn glob_expand(input: &str) -> Result<Vec<PathBuf>> {
    let paths = glob::glob(input)?.collect::<Result<Vec<_>, _>>()?;
    if paths.is_empty() {
        return Err(anyhow!("No files found matching pattern: '{}'", input));
    }
    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    fn setup_test_env() -> Result<tempfile::TempDir> {
        let temp_dir = tempfile::tempdir()?;
        let test_dir = temp_dir.path().join("test_data");
        fs::create_dir(&test_dir)?;

        fs::File::create(test_dir.join("game.pfs"))?;
        fs::File::create(test_dir.join("game.pfs.000"))?;

        let mut f = fs::File::create(test_dir.join("readme.txt"))?;
        f.write_all(b"test content")?;

        fs::create_dir(test_dir.join("assets"))?;
        Ok(temp_dir)
    }

    #[test]
    fn test_is_file_pf8_from_filename() {
        assert!(is_file_pf8_from_filename(Path::new("game.pfs")));
        assert!(is_file_pf8_from_filename(Path::new("test.pfs.000")));
        assert!(is_file_pf8_from_filename(Path::new("/path/to/file.pfs")));
        assert!(!is_file_pf8_from_filename(Path::new("readme.txt")));
        assert!(!is_file_pf8_from_filename(Path::new("game.zip")));
    }

    #[test]
    fn test_get_pfs_basename() {
        assert_eq!(get_pfs_basename(Path::new("game.pfs")).unwrap(), "game");
        assert_eq!(get_pfs_basename(Path::new("test.pfs.000")).unwrap(), "test");
        assert_eq!(
            get_pfs_basename(Path::new("/path/to/file.pfs")).unwrap(),
            "file"
        );
        assert_eq!(
            get_pfs_basename(Path::new("normal.txt")).unwrap(),
            "normal.txt"
        );
    }

    #[test]
    fn test_get_pfs_basepath() -> Result<()> {
        assert_eq!(
            get_pfs_basepath(Path::new("/test/dir/game.pfs"))?,
            PathBuf::from("/test/dir/game")
        );
        assert_eq!(
            get_pfs_basepath(Path::new("/test/dir/game.pfs.000"))?,
            PathBuf::from("/test/dir/game")
        );
        Ok(())
    }

    #[test]
    fn test_process_cli_inputs_pfs_only() -> Result<()> {
        let temp_dir = setup_test_env()?;
        let pfs_file1 = temp_dir.path().join("test_data").join("game.pfs");
        let pfs_file2 = temp_dir.path().join("test_data").join("game.pfs.000");

        match process_cli_inputs(vec![pfs_file1.clone(), pfs_file2.clone()])? {
            InputType::PfsFiles(files) => {
                assert_eq!(files, vec![pfs_file1, pfs_file2]);
            }
            _ => panic!("Expected PfsFiles variant"),
        }
        Ok(())
    }

    #[test]
    fn test_process_cli_inputs_pack_files() -> Result<()> {
        let temp_dir = setup_test_env()?;
        let test_dir = temp_dir.path().join("test_data");
        let normal_file = test_dir.join("readme.txt");
        let sub_dir = test_dir.join("assets");

        match process_cli_inputs(vec![normal_file.clone(), sub_dir.clone()])? {
            InputType::PackFiles { dirs, files } => {
                assert_eq!(dirs, vec![sub_dir]);
                assert_eq!(files, vec![normal_file]);
            }
            _ => panic!("Expected PackFiles variant"),
        }
        Ok(())
    }

    #[test]
    fn test_process_cli_inputs_mixed_error() -> Result<()> {
        let temp_dir = setup_test_env()?;
        let test_dir = temp_dir.path().join("test_data");
        let result =
            process_cli_inputs(vec![test_dir.join("game.pfs"), test_dir.join("readme.txt")]);
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Cannot mix PFS files")
        );
        Ok(())
    }

    #[test]
    fn test_process_cli_inputs_empty_error() {
        assert!(
            process_cli_inputs(vec![])
                .unwrap_err()
                .to_string()
                .contains("No input provided")
        );
    }

    #[test]
    fn test_process_cli_inputs_nonexistent_path() {
        assert!(
            process_cli_inputs(vec![PathBuf::from("/nonexistent/path")])
                .unwrap_err()
                .to_string()
                .contains("does not exist")
        );
    }
}
