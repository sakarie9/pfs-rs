// Library interface for pfs-rs
// This allows tests and potentially other crates to use internal functions

pub mod util;

// Re-export functions from main for testing
use anyhow::Result;
use std::path::{Path, PathBuf};

/// Determine output directory for extraction
pub fn determine_extract_output(
    input: &Path,
    specified_output: Option<&Path>,
    separate: bool,
) -> PathBuf {
    if let Some(output) = specified_output {
        if separate {
            // If separate mode, append input filename to output
            let filename = input
                .file_stem()
                .unwrap_or_else(|| input.file_name().unwrap());
            output.join(filename)
        } else {
            output.to_path_buf()
        }
    } else {
        // Auto-detect: extract to directory with same name as archive (without .pfs)
        util::get_pfs_basepath(input).unwrap_or_else(|_| input.with_extension(""))
    }
}

/// Determine output file for packing
pub fn determine_pack_output(
    _inputs: &[PathBuf],
    specified_output: Option<&Path>,
    overwrite: bool,
) -> Result<PathBuf> {
    if let Some(output) = specified_output {
        if output.is_dir() {
            // Output is a directory, always use "root" as base name
            if overwrite {
                Ok(output.join("root.pfs"))
            } else {
                util::try_get_next_nonexist_pfs(output, "root")
            }
        } else {
            Ok(output.to_path_buf())
        }
    } else {
        // Auto-detect: create "root.pfs" in current directory
        let current_dir = std::env::current_dir()?;

        if overwrite {
            Ok(current_dir.join("root.pfs"))
        } else {
            util::try_get_next_nonexist_pfs(&current_dir, "root")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_extract_output() {
        let input = Path::new("game.pfs");
        let output = determine_extract_output(input, None, false);
        assert_eq!(output, PathBuf::from("game"));
    }

    #[test]
    fn test_basic_pack_output() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let inputs = vec![PathBuf::from("somedir")];

        // Change to temp dir
        let original = std::env::current_dir()?;
        std::env::set_current_dir(temp_dir.path())?;

        let output = determine_pack_output(&inputs, None, true)?;
        assert_eq!(output.file_name().unwrap(), "root.pfs");

        // Restore
        std::env::set_current_dir(original)?;
        Ok(())
    }
}
