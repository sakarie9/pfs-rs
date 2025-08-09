use anyhow::{Result, anyhow};
use std::path::{Path, PathBuf};

pub fn is_file_pf8_from_filename(path: &Path) -> bool {
    if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
        if name.contains(".pfs") {
            return true;
        }
        false
    } else {
        false
    }
}

pub fn glob_expand(input: &str) -> Result<Vec<PathBuf>> {
    let paths = glob::glob(input)?.collect::<Result<Vec<_>, _>>()?;
    if paths.is_empty() {
        return Err(anyhow!("No files found matching pattern: '{}'", input));
    }
    Ok(paths)
}

/// Extracts the base name of a file without the ".pfs" extension.
///
/// # Arguments
///
/// * `input` - A reference to a `Path` representing the file path.
///
/// # Returns
///
/// * `Ok(String)` - The base name of the file without the ".pfs" extension if successful.
/// * `Err(anyhow::Error)` - If the file name is invalid or does not contain the ".pfs" extension.
pub fn get_pfs_basename(input: &Path) -> Result<String> {
    if let Some(name) = input.file_name().and_then(|s| s.to_str()) {
        if let Some(pos) = name.find(".pfs") {
            return Ok(name[..pos].to_string());
        }
        return Ok(name.to_string());
    }
    Err(anyhow!("Failed to get file name"))
}

pub fn get_pfs_basepath(input: &Path) -> Result<PathBuf> {
    if let Some(name) = input.file_name().and_then(|s| s.to_str()) {
        if let Some(pos) = name.find(".pfs") {
            let base = input.parent().unwrap();
            let path = base.join(&name[..pos]);
            return Ok(path);
        }
        return Err(anyhow!("Invalid file name"));
    }
    Err(anyhow!("Failed to get file name"))
}

/// input: dir: workdir/test base: root
/// output: Ok(workdir/test/root.pfs.000)
pub fn try_get_next_nonexist_pfs(dir: &Path, base: &str) -> Result<PathBuf> {
    // return root.pfs if not exist
    let filename = format!("{base}.pfs");
    let path = dir.join(filename);
    if !path.exists() {
        return Ok(path);
    }
    // return root.pfs.xxx if not exist
    let mut i = 0;
    loop {
        let filename = format!("{base}.pfs.{i:03}");
        let path = dir.join(filename);
        if !path.exists() {
            return Ok(path);
        }
        i += 1;
    }
}

/// 输入类型枚举
#[derive(Debug, Clone)]
pub enum InputType {
    PfsFiles(Vec<PathBuf>),
    PackFiles {
        dirs: Vec<PathBuf>,
        files: Vec<PathBuf>,
    },
}

/// 输入处理结果
#[derive(Debug)]
pub struct InputProcessResult {
    pub input_type: InputType,
    pub suggested_output: Option<PathBuf>,
}

/// 处理多种形式的CLI输入路径
pub fn process_cli_inputs(inputs: Vec<PathBuf>) -> Result<InputProcessResult> {
    if inputs.is_empty() {
        return Err(anyhow!("No input provided"));
    }

    let mut pfs_files = Vec::new();
    let mut directories = Vec::new();
    let mut regular_files = Vec::new();

    // 分类输入
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

    // 根据输入类型确定操作
    let has_pfs = !pfs_files.is_empty();
    let has_pack_input = !directories.is_empty() || !regular_files.is_empty();

    match (has_pfs, has_pack_input) {
        (true, false) => {
            // 只有 PFS 文件，执行解包操作
            let suggested_output = get_pfs_basepath(&pfs_files[0]).ok();
            Ok(InputProcessResult {
                input_type: InputType::PfsFiles(pfs_files),
                suggested_output,
            })
        }
        (false, true) => {
            // 只有目录或文件，执行打包操作
            let base_dir = if !directories.is_empty() {
                directories[0].parent().map(|p| p.to_path_buf())
            } else {
                regular_files[0].parent().map(|p| p.to_path_buf())
            };

            let suggested_output = base_dir.map(|dir| dir.join("root.pfs"));

            Ok(InputProcessResult {
                input_type: InputType::PackFiles {
                    dirs: directories,
                    files: regular_files,
                },
                suggested_output,
            })
        }
        (true, true) => Err(anyhow!(
            "Cannot mix PFS files and pack inputs (directories/files) in the same operation"
        )),
        (false, false) => Err(anyhow!("No valid input found")),
    }
}

/// 根据overwrite标志获取最终输出路径
pub fn get_final_output_path(suggested_output: PathBuf, overwrite: bool) -> Result<PathBuf> {
    if overwrite {
        Ok(suggested_output)
    } else {
        // 如果是.pfs文件，尝试找到不存在的文件名
        if let Some(parent) = suggested_output.parent() {
            if let Some(stem) = suggested_output.file_stem().and_then(|s| s.to_str()) {
                return try_get_next_nonexist_pfs(parent, stem);
            }
        }
        Ok(suggested_output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    /// 创建临时测试目录结构
    fn setup_test_env() -> Result<tempfile::TempDir> {
        let temp_dir = tempfile::tempdir()?;

        // 创建一些测试文件和目录
        let test_dir = temp_dir.path().join("test_data");
        fs::create_dir(&test_dir)?;

        // 创建一个 PFS 文件
        let pfs_file = test_dir.join("game.pfs");
        fs::File::create(&pfs_file)?;

        let pfs_file = test_dir.join("game.pfs.000");
        fs::File::create(&pfs_file)?;

        // 创建一个普通文件
        let normal_file = test_dir.join("readme.txt");
        let mut file = fs::File::create(&normal_file)?;
        file.write_all(b"test content")?;

        // 创建一个子目录
        let sub_dir = test_dir.join("assets");
        fs::create_dir(&sub_dir)?;

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
        let pfs_path = Path::new("/test/dir/game.pfs");
        let result = get_pfs_basepath(pfs_path)?;
        assert_eq!(result, PathBuf::from("/test/dir/game"));

        let pfs_numbered = Path::new("/test/dir/game.pfs.000");
        let result2 = get_pfs_basepath(pfs_numbered)?;
        assert_eq!(result2, PathBuf::from("/test/dir/game"));

        Ok(())
    }

    #[test]
    fn test_process_cli_inputs_pfs_only() -> Result<()> {
        let temp_dir = setup_test_env()?;
        let pfs_file1 = temp_dir.path().join("test_data").join("game.pfs");
        let pfs_file2 = temp_dir.path().join("test_data").join("game.pfs.000");

        let result = process_cli_inputs(vec![pfs_file1.clone(), pfs_file2.clone()])?;

        match result.input_type {
            InputType::PfsFiles(files) => {
                assert_eq!(files.len(), 2);
                assert_eq!(files[0], pfs_file1);
                assert_eq!(files[1], pfs_file2);
            }
            _ => panic!("Expected PfsFiles variant"),
        }

        assert!(result.suggested_output.is_some());
        Ok(())
    }

    #[test]
    fn test_process_cli_inputs_pack_files() -> Result<()> {
        let temp_dir = setup_test_env()?;
        let test_dir = temp_dir.path().join("test_data");
        let normal_file = test_dir.join("readme.txt");
        let sub_dir = test_dir.join("assets");

        let result = process_cli_inputs(vec![normal_file.clone(), sub_dir.clone()])?;

        match result.input_type {
            InputType::PackFiles { dirs, files } => {
                assert_eq!(dirs.len(), 1);
                assert_eq!(files.len(), 1);
                assert_eq!(dirs[0], sub_dir);
                assert_eq!(files[0], normal_file);
            }
            _ => panic!("Expected PackFiles variant"),
        }

        assert!(result.suggested_output.is_some());
        let suggested = result.suggested_output.unwrap();
        assert_eq!(suggested.file_name().unwrap(), "root.pfs");
        Ok(())
    }

    #[test]
    fn test_process_cli_inputs_mixed_error() -> Result<()> {
        let temp_dir = setup_test_env()?;
        let test_dir = temp_dir.path().join("test_data");
        let pfs_file = test_dir.join("game.pfs");
        let normal_file = test_dir.join("readme.txt");

        let result = process_cli_inputs(vec![pfs_file, normal_file]);
        assert!(result.is_err());
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
        let result = process_cli_inputs(vec![]);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("No input provided")
        );
    }

    #[test]
    fn test_process_cli_inputs_nonexistent_path() {
        let nonexistent = PathBuf::from("/nonexistent/path");
        let result = process_cli_inputs(vec![nonexistent]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));
    }

    #[test]
    fn test_get_final_output_path_overwrite() -> Result<()> {
        let suggested = PathBuf::from("/test/output.pfs");
        let result = get_final_output_path(suggested.clone(), true)?;
        assert_eq!(result, suggested);
        Ok(())
    }

    #[test]
    fn test_get_final_output_path_no_overwrite() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let suggested = temp_dir.path().join("test.pfs");

        let result = get_final_output_path(suggested.clone(), false)?;
        // 因为文件不存在，应该返回原始路径
        assert_eq!(result, suggested);

        // 创建文件后，应该返回不同的路径
        fs::File::create(&suggested)?;
        let result2 = get_final_output_path(suggested.clone(), false)?;
        assert_ne!(result2, suggested);
        assert!(result2.to_string_lossy().contains("test.pfs.000"));

        Ok(())
    }
}
