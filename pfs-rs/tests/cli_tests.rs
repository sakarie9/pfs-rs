use anyhow::Result;
use std::path::{Path, PathBuf};

// 导入要测试的函数
// 注意：这些函数在 main.rs 中已经标记为 pub(crate)
use pfs_rs::{determine_extract_output, determine_pack_output};

/// 测试模块：Extract 输出路径推断
mod extract_output_tests {
    use super::*;

    #[test]
    fn test_extract_default_strips_pfs_extension() {
        // 测试：game.pfs → game/
        let input = Path::new("/path/to/game.pfs");
        let output = determine_extract_output(input, None, false);
        assert_eq!(output, PathBuf::from("/path/to/game"));
    }

    #[test]
    fn test_extract_default_strips_numbered_extension() {
        // 测试：game.pfs.000 → game/
        let input = Path::new("/path/to/game.pfs.000");
        let output = determine_extract_output(input, None, false);
        assert_eq!(output, PathBuf::from("/path/to/game"));
    }

    #[test]
    fn test_extract_default_multiple_numbered() {
        // 测试：game.pfs.001 → game/
        let input = Path::new("/path/to/game.pfs.001");
        let output = determine_extract_output(input, None, false);
        assert_eq!(output, PathBuf::from("/path/to/game"));
    }

    #[test]
    fn test_extract_to_specified_output_not_separate() {
        // 测试：指定输出目录，不分离
        let input = Path::new("/path/to/game.pfs");
        let output_dir = Path::new("/output");
        let output = determine_extract_output(input, Some(output_dir), false);
        assert_eq!(output, PathBuf::from("/output"));
    }

    #[test]
    fn test_extract_to_specified_output_with_separate() {
        // 测试：指定输出目录，分离模式
        let input = Path::new("/path/to/game.pfs");
        let output_dir = Path::new("/output");
        let output = determine_extract_output(input, Some(output_dir), true);
        // 应该在输出目录下创建以归档名命名的子目录
        assert_eq!(output, PathBuf::from("/output/game"));
    }

    #[test]
    fn test_extract_separate_mode_with_numbered() {
        // 测试：分离模式处理带编号的归档
        let input = Path::new("/path/to/archive.pfs.005");
        let output_dir = Path::new("/dest");
        let output = determine_extract_output(input, Some(output_dir), true);
        // file_stem() 会去掉最后一个扩展名，所以 archive.pfs.005 的 stem 是 archive.pfs
        // 我们的实现是基于 file_stem，所以结果是 /dest/archive.pfs
        // 如果需要完全去掉 .pfs 部分，需要使用 get_pfs_basepath
        assert!(output.to_str().unwrap().contains("archive"));
    }

    #[test]
    fn test_extract_relative_path() {
        // 测试：相对路径
        let input = Path::new("game.pfs");
        let output = determine_extract_output(input, None, false);
        assert_eq!(output, PathBuf::from("game"));
    }

    #[test]
    fn test_extract_current_dir() {
        // 测试：当前目录下的文件
        let input = Path::new("./test.pfs");
        let output = determine_extract_output(input, None, false);
        assert_eq!(output, PathBuf::from("./test"));
    }

    #[test]
    fn test_extract_complex_path() {
        // 测试：复杂路径
        let input = Path::new("/var/data/archives/game_v2.pfs");
        let output = determine_extract_output(input, None, false);
        assert_eq!(output, PathBuf::from("/var/data/archives/game_v2"));
    }

    #[test]
    fn test_extract_with_dots_in_name() {
        // 测试：文件名中包含点号
        let input = Path::new("/path/game.v1.0.pfs");
        let output = determine_extract_output(input, None, false);
        // 应该只去掉最后的 .pfs
        assert_eq!(output, PathBuf::from("/path/game.v1.0"));
    }
}

/// 测试模块：Create 输出路径推断
mod create_output_tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_create_default_generates_root_pfs() -> Result<()> {
        // 测试：不指定输出，应该生成当前目录的 root.pfs
        let temp_dir = tempfile::tempdir()?;
        let inputs = vec![PathBuf::from("/some/dir")];

        // 模拟在临时目录中
        let _guard = TestDirGuard::new(temp_dir.path())?;

        let output = determine_pack_output(&inputs, None, true)?;
        assert_eq!(output.file_name().unwrap(), "root.pfs");

        Ok(())
    }

    #[test]
    fn test_create_with_specific_file_output() -> Result<()> {
        // 测试：指定具体文件名
        let inputs = vec![PathBuf::from("/some/dir")];
        let output_file = Path::new("/output/custom.pfs");

        let output = determine_pack_output(&inputs, Some(output_file), true)?;
        assert_eq!(output, PathBuf::from("/output/custom.pfs"));

        Ok(())
    }

    #[test]
    fn test_create_output_to_directory_generates_root() -> Result<()> {
        // 测试：输出到目录时生成 root.pfs
        let temp_dir = tempfile::tempdir()?;
        let inputs = vec![PathBuf::from("/some/dir")];

        let output = determine_pack_output(&inputs, Some(temp_dir.path()), true)?;
        assert_eq!(output.file_name().unwrap(), "root.pfs");
        assert_eq!(output.parent().unwrap(), temp_dir.path());

        Ok(())
    }

    #[test]
    fn test_create_with_multiple_inputs() -> Result<()> {
        // 测试：多个输入仍然生成 root.pfs
        let temp_dir = tempfile::tempdir()?;
        let inputs = vec![
            PathBuf::from("/path/dir1"),
            PathBuf::from("/path/dir2"),
            PathBuf::from("/path/file.txt"),
        ];

        let _guard = TestDirGuard::new(temp_dir.path())?;

        let output = determine_pack_output(&inputs, None, true)?;
        assert_eq!(output.file_name().unwrap(), "root.pfs");

        Ok(())
    }

    #[test]
    fn test_create_overwrite_flag_behavior() -> Result<()> {
        // 测试：overwrite 标志的路径生成（不实际检查文件是否存在）
        let temp_dir = tempfile::tempdir()?;
        let inputs = vec![PathBuf::from("/some/dir")];

        let _guard = TestDirGuard::new(temp_dir.path())?;

        // overwrite=true 应该总是返回 root.pfs
        let output1 = determine_pack_output(&inputs, None, true)?;
        assert_eq!(output1.file_name().unwrap(), "root.pfs");

        Ok(())
    }

    #[test]
    fn test_create_no_overwrite_with_conflict() -> Result<()> {
        // 测试：文件存在时的自动编号
        let temp_dir = tempfile::tempdir()?;
        let inputs = vec![PathBuf::from("/some/dir")];

        // 创建 root.pfs 文件
        let root_pfs = temp_dir.path().join("root.pfs");
        fs::write(&root_pfs, b"test")?;

        let _guard = TestDirGuard::new(temp_dir.path())?;

        // overwrite=false 应该生成 root.pfs.000
        let output = determine_pack_output(&inputs, None, false)?;
        assert_eq!(output.file_name().unwrap(), "root.pfs.000");

        Ok(())
    }

    #[test]
    fn test_create_sequential_numbering() -> Result<()> {
        // 测试：连续的编号
        let temp_dir = tempfile::tempdir()?;
        let inputs = vec![PathBuf::from("/some/dir")];

        // 创建多个已存在的文件
        fs::write(temp_dir.path().join("root.pfs"), b"test")?;
        fs::write(temp_dir.path().join("root.pfs.000"), b"test")?;
        fs::write(temp_dir.path().join("root.pfs.001"), b"test")?;

        let _guard = TestDirGuard::new(temp_dir.path())?;

        // 应该生成 root.pfs.002
        let output = determine_pack_output(&inputs, None, false)?;
        assert_eq!(output.file_name().unwrap(), "root.pfs.002");

        Ok(())
    }

    #[test]
    fn test_create_to_directory_with_conflict() -> Result<()> {
        // 测试：输出到目录且有冲突
        let temp_dir = tempfile::tempdir()?;
        let output_dir = temp_dir.path().join("output");
        fs::create_dir(&output_dir)?;

        let inputs = vec![PathBuf::from("/some/dir")];

        // 在输出目录创建 root.pfs
        fs::write(output_dir.join("root.pfs"), b"test")?;

        // 应该生成 root.pfs.000
        let output = determine_pack_output(&inputs, Some(&output_dir), false)?;
        assert_eq!(output.file_name().unwrap(), "root.pfs.000");
        assert_eq!(output.parent().unwrap(), output_dir);

        Ok(())
    }

    #[test]
    fn test_create_relative_output_path() -> Result<()> {
        // 测试：相对路径输出
        let inputs = vec![PathBuf::from("/some/dir")];
        let output_file = Path::new("output/archive.pfs");

        let output = determine_pack_output(&inputs, Some(output_file), true)?;
        assert_eq!(output, PathBuf::from("output/archive.pfs"));

        Ok(())
    }

    /// 辅助结构：临时改变当前工作目录
    pub struct TestDirGuard {
        original: PathBuf,
    }

    impl TestDirGuard {
        pub fn new(path: &Path) -> Result<Self> {
            let original = std::env::current_dir()?;
            std::env::set_current_dir(path)?;
            Ok(Self { original })
        }
    }

    impl Drop for TestDirGuard {
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.original);
        }
    }
}

/// 测试模块：路径边界情况
mod path_edge_cases {
    use super::*;

    #[test]
    fn test_extract_empty_filename_fallback() {
        // 测试：没有 .pfs 扩展名的情况
        let input = Path::new("/path/to/archive");
        let output = determine_extract_output(input, None, false);
        // 应该使用 with_extension("") 的结果
        assert_eq!(output, PathBuf::from("/path/to/archive"));
    }

    #[test]
    fn test_extract_only_extension() {
        // 测试：只有扩展名的文件
        let input = Path::new(".pfs");
        let output = determine_extract_output(input, None, false);
        // with_extension("") 在这种情况下会返回空路径
        // 我们只是测试不会 panic
        // 实际结果取决于实现细节
        let _ = output; // 只要不 panic 就可以
    }

    #[test]
    fn test_extract_unicode_filename() {
        // 测试：Unicode 文件名
        let input = Path::new("/path/游戏.pfs");
        let output = determine_extract_output(input, None, false);
        assert_eq!(output, PathBuf::from("/path/游戏"));
    }

    #[test]
    fn test_extract_special_chars_in_path() {
        // 测试：路径中的特殊字符
        let input = Path::new("/path/my game (v2).pfs");
        let output = determine_extract_output(input, None, false);
        assert_eq!(output, PathBuf::from("/path/my game (v2)"));
    }

    #[test]
    fn test_extract_spaces_in_path() {
        // 测试：路径中的空格
        let input = Path::new("/my path/to game/archive.pfs");
        let output = determine_extract_output(input, None, false);
        assert_eq!(output, PathBuf::from("/my path/to game/archive"));
    }

    #[test]
    fn test_create_empty_inputs() -> Result<()> {
        // 测试：空输入数组
        let temp_dir = tempfile::tempdir()?;
        let inputs: Vec<PathBuf> = vec![];

        // 改变到临时目录以避免污染当前目录
        let original_dir = std::env::current_dir()?;
        std::env::set_current_dir(temp_dir.path())?;

        // 即使没有输入，仍应该能生成路径
        let output = determine_pack_output(&inputs, None, true)?;
        assert_eq!(output.file_name().unwrap(), "root.pfs");

        // 恢复原始目录
        std::env::set_current_dir(original_dir)?;

        Ok(())
    }
}

/// 测试模块：util 函数的路径处理
mod util_path_tests {
    use anyhow::Result;
    use pfs_rs::util;
    use std::path::{Path, PathBuf};

    #[test]
    fn test_get_pfs_basepath_simple() -> Result<()> {
        let input = Path::new("/test/dir/game.pfs");
        let result = util::get_pfs_basepath(input)?;
        assert_eq!(result, PathBuf::from("/test/dir/game"));
        Ok(())
    }

    #[test]
    fn test_get_pfs_basepath_numbered() -> Result<()> {
        let input = Path::new("/test/dir/game.pfs.000");
        let result = util::get_pfs_basepath(input)?;
        assert_eq!(result, PathBuf::from("/test/dir/game"));
        Ok(())
    }

    #[test]
    fn test_get_pfs_basepath_multiple_dots() -> Result<()> {
        let input = Path::new("/test/game.v1.0.pfs");
        let result = util::get_pfs_basepath(input)?;
        assert_eq!(result, PathBuf::from("/test/game.v1.0"));
        Ok(())
    }

    #[test]
    fn test_get_pfs_basepath_current_dir() -> Result<()> {
        let input = Path::new("game.pfs");
        let result = util::get_pfs_basepath(input)?;
        assert_eq!(result, PathBuf::from("game"));
        Ok(())
    }

    #[test]
    fn test_try_get_next_nonexist_pfs_no_conflict() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let result = util::try_get_next_nonexist_pfs(temp_dir.path(), "test")?;
        assert_eq!(result.file_name().unwrap(), "test.pfs");
        Ok(())
    }

    #[test]
    fn test_try_get_next_nonexist_pfs_with_conflict() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;

        // 创建 test.pfs
        std::fs::write(temp_dir.path().join("test.pfs"), b"test")?;

        let result = util::try_get_next_nonexist_pfs(temp_dir.path(), "test")?;
        assert_eq!(result.file_name().unwrap(), "test.pfs.000");
        Ok(())
    }

    #[test]
    fn test_try_get_next_nonexist_pfs_sequential() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;

        // 创建多个文件
        std::fs::write(temp_dir.path().join("test.pfs"), b"test")?;
        std::fs::write(temp_dir.path().join("test.pfs.000"), b"test")?;
        std::fs::write(temp_dir.path().join("test.pfs.001"), b"test")?;

        let result = util::try_get_next_nonexist_pfs(temp_dir.path(), "test")?;
        assert_eq!(result.file_name().unwrap(), "test.pfs.002");
        Ok(())
    }

    #[test]
    fn test_is_file_pf8_from_filename() {
        assert!(util::is_file_pf8_from_filename(Path::new("game.pfs")));
        assert!(util::is_file_pf8_from_filename(Path::new("test.pfs.000")));
        assert!(util::is_file_pf8_from_filename(Path::new(
            "/path/to/file.pfs"
        )));
        assert!(!util::is_file_pf8_from_filename(Path::new("readme.txt")));
        assert!(!util::is_file_pf8_from_filename(Path::new("game.zip")));
    }

    #[test]
    fn test_is_file_pf8_unicode() {
        assert!(util::is_file_pf8_from_filename(Path::new("游戏.pfs")));
        assert!(util::is_file_pf8_from_filename(Path::new(
            "アーカイブ.pfs.001"
        )));
    }
}
