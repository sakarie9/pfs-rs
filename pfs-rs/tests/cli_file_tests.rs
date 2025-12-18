#[cfg(test)]
mod pack_unpack_integration_tests {
    use assert_cmd::cargo::cargo_bin_cmd;
    use assert_fs::prelude::*;
    use predicates::prelude::*;

    #[test]
    #[ignore = "Ignored by default because it involves filesystem and process operations"]
    fn test_pack_unpack_basic_structure() -> anyhow::Result<()> {
        // 创建临时目录
        let temp = assert_fs::TempDir::new()?;

        // 创建测试目录结构:
        // source/
        // ├── file1.txt (content: "content1")
        // └── subdir/
        //     └── file2.txt (content: "content2")
        let source = temp.child("source");
        source.create_dir_all()?;
        source.child("file1.txt").write_str("content1")?;
        source
            .child("subdir")
            .child("file2.txt")
            .write_str("content2")?;

        let archive = temp.child("test.pfs");

        // 执行打包命令:
        // pfs-rs c source/ -o test.pfs -q
        cargo_bin_cmd!("pfs-rs")
            .arg("c")
            .arg("source/")
            .arg("-o")
            .arg(archive.path())
            .arg("-q")
            .current_dir(temp.path())
            .assert()
            .success();

        // 验证归档文件已创建
        archive.assert(predicate::path::exists());

        // 创建解包目录
        let extract = temp.child("extracted");
        extract.create_dir_all()?;

        // 执行解包命令:
        // pfs-rs x test.pfs extracted/ -q
        cargo_bin_cmd!("pfs-rs")
            .arg("x")
            .arg(archive.path())
            .arg(extract.path())
            .arg("-q")
            .assert()
            .success();

        // 验证解包后的目录结构:
        // extracted/
        // ├── file1.txt (不包含 source/ 目录)
        // └── subdir/
        //     └── file2.txt
        extract
            .child("file1.txt")
            .assert(predicate::path::exists())
            .assert(predicate::str::contains("content1"));

        extract
            .child("subdir/file2.txt")
            .assert(predicate::path::exists())
            .assert(predicate::str::contains("content2"));

        Ok(())
    }

    #[test]
    #[ignore = "Ignored by default because it involves filesystem and process operations"]
    fn test_pack_with_dir_preservation() -> anyhow::Result<()> {
        // 创建临时目录
        let temp = assert_fs::TempDir::new()?;

        // 创建测试目录结构:
        // game/
        // └── data.txt (content: "test data")
        let game = temp.child("game");
        game.create_dir_all()?;
        game.child("data.txt").write_str("test data")?;

        let archive = temp.child("with_dir.pfs");

        // 执行打包命令:
        // pfs-rs c game -o with_dir.pfs -q
        cargo_bin_cmd!("pfs-rs")
            .arg("c")
            .arg("game")
            .arg("-o")
            .arg(archive.path())
            .arg("-q")
            .current_dir(temp.path())
            .assert()
            .success();

        // 创建解包目录
        let extract = temp.child("extracted");
        extract.create_dir_all()?;

        // 执行解包命令:
        // pfs-rs x with_dir.pfs extracted/ -q
        cargo_bin_cmd!("pfs-rs")
            .arg("x")
            .arg(archive.path())
            .arg(extract.path())
            .arg("-q")
            .assert()
            .success();

        // 验证解包后的目录结构:
        // extracted/
        // └── game/          (保留了目录名)
        //     └── data.txt
        extract
            .child("game/data.txt")
            .assert(predicate::path::exists())
            .assert(predicate::path::is_file())
            .assert(predicate::str::contains("test data"));

        Ok(())
    }

    #[test]
    #[ignore = "Ignored by default because it involves filesystem and process operations"]
    fn test_pack_without_dir_preservation() -> anyhow::Result<()> {
        // 创建临时目录
        let temp = assert_fs::TempDir::new()?;

        // 创建测试目录结构:
        // game/
        // ├── config.ini (content: "config")
        // └── scripts/
        //     └── main.txt (content: "script")
        let game = temp.child("game");
        game.create_dir_all()?;
        game.child("config.ini").write_str("config")?;
        game.child("scripts")
            .child("main.txt")
            .write_str("script")?;

        let archive = temp.child("without_dir.pfs");

        // 执行打包命令:
        // pfs-rs c game/ -o without_dir.pfs -q
        cargo_bin_cmd!("pfs-rs")
            .arg("c")
            .arg("game/")
            .arg("-o")
            .arg(archive.path())
            .arg("-q")
            .current_dir(temp.path())
            .assert()
            .success();

        // 创建解包目录
        let extract = temp.child("extracted");
        extract.create_dir_all()?;

        // 执行解包命令:
        // pfs-rs x without_dir.pfs extracted/ -q
        cargo_bin_cmd!("pfs-rs")
            .arg("x")
            .arg(archive.path())
            .arg(extract.path())
            .arg("-q")
            .assert()
            .success();

        // 验证解包后的目录结构:
        // extracted/
        // ├── config.ini      (不包含 game/ 目录)
        // └── scripts/
        //     └── main.txt
        extract
            .child("config.ini")
            .assert(predicate::path::exists());

        extract
            .child("scripts/main.txt")
            .assert(predicate::path::exists());

        // 确认 game/ 目录不存在
        extract
            .child("game")
            .assert(predicate::path::exists().not());

        Ok(())
    }

    #[test]
    #[ignore = "Ignored by default because it involves filesystem and process operations"]
    fn test_pack_multiple_dirs() -> anyhow::Result<()> {
        // 创建临时目录
        let temp = assert_fs::TempDir::new()?;

        // 创建测试目录结构:
        // a/
        // └── file_a.txt (content: "content a")
        // b/
        // └── file_b.txt (content: "content b")
        let dir_a = temp.child("a");
        dir_a.create_dir_all()?;
        dir_a.child("file_a.txt").write_str("content a")?;

        let dir_b = temp.child("b");
        dir_b.create_dir_all()?;
        dir_b.child("file_b.txt").write_str("content b")?;

        let archive = temp.child("multi.pfs");

        // 执行打包命令:
        // pfs-rs c a b -o multi.pfs -q
        cargo_bin_cmd!("pfs-rs")
            .arg("c")
            .arg("a")
            .arg("b")
            .arg("-o")
            .arg(archive.path())
            .arg("-q")
            .current_dir(temp.path())
            .assert()
            .success();

        // 创建解包目录
        let extract = temp.child("extracted");
        extract.create_dir_all()?;

        // 执行解包命令:
        // pfs-rs x multi.pfs extracted/ -q
        cargo_bin_cmd!("pfs-rs")
            .arg("x")
            .arg(archive.path())
            .arg(extract.path())
            .arg("-q")
            .assert()
            .success();

        // 验证解包后的目录结构:
        // extracted/
        // ├── a/             (保留两个目录)
        // │   └── file_a.txt
        // └── b/
        //     └── file_b.txt
        extract
            .child("a/file_a.txt")
            .assert(predicate::path::exists())
            .assert(predicate::str::contains("content a"));

        extract
            .child("b/file_b.txt")
            .assert(predicate::path::exists())
            .assert(predicate::str::contains("content b"));

        Ok(())
    }

    #[test]
    #[ignore = "Ignored by default because it involves filesystem and process operations"]
    fn test_system_ini_detection() -> anyhow::Result<()> {
        // 创建临时目录
        let temp = assert_fs::TempDir::new()?;

        // 创建经典游戏目录结构:
        // game/
        // ├── system.ini (content: "[Game]\nTitle=Test\n")
        // └── script/
        //     └── start.txt (content: "script content")
        let game = temp.child("game");
        game.create_dir_all()?;
        game.child("system.ini").write_str("[Game]\nTitle=Test\n")?;
        game.child("script")
            .child("start.txt")
            .write_str("script content")?;

        let archive = temp.child("game.pfs");

        // 执行打包命令:
        // pfs-rs c game -o game.pfs -q
        //
        // 智能检测: 检测到目录中有 system.ini，自动去掉目录层级
        cargo_bin_cmd!("pfs-rs")
            .arg("c")
            .arg("game")
            .arg("-o")
            .arg(archive.path())
            .arg("-q")
            .current_dir(temp.path())
            .assert()
            .success();

        // 创建解包目录
        let extract = temp.child("extracted");
        extract.create_dir_all()?;

        // 执行解包命令:
        // pfs-rs x game.pfs extracted/ -q
        cargo_bin_cmd!("pfs-rs")
            .arg("x")
            .arg(archive.path())
            .arg(extract.path())
            .arg("-q")
            .assert()
            .success();

        // 验证解包后的目录结构:
        // extracted/
        // ├── system.ini     (智能检测生效，不包含 game/ 目录)
        // └── script/
        //     └── start.txt
        extract
            .child("system.ini")
            .assert(predicate::path::exists());

        extract
            .child("script/start.txt")
            .assert(predicate::path::exists());

        // 确认 game/ 目录不存在(智能检测生效)
        extract
            .child("game")
            .assert(predicate::path::exists().not());

        Ok(())
    }

    #[test]
    #[ignore = "Ignored by default because it involves filesystem and process operations"]
    fn test_system_ini_detection_disabled() -> anyhow::Result<()> {
        // 创建临时目录
        let temp = assert_fs::TempDir::new()?;

        // 创建经典游戏目录结构:
        // game/
        // └── system.ini (content: "[Game]\n")
        let game = temp.child("game");
        game.create_dir_all()?;
        game.child("system.ini").write_str("[Game]\n")?;

        let archive = temp.child("game.pfs");

        // 执行打包命令:
        // pfs-rs c game -o game.pfs --no-smart-detect -q
        //
        // 智能检测: 被 --no-smart-detect 禁用，保留 game/ 目录层级
        cargo_bin_cmd!("pfs-rs")
            .arg("c")
            .arg("game")
            .arg("-o")
            .arg(archive.path())
            .arg("--no-smart-detect")
            .arg("-q")
            .current_dir(temp.path())
            .assert()
            .success();

        // 创建解包目录
        let extract = temp.child("extracted");
        extract.create_dir_all()?;

        // 执行解包命令:
        // pfs-rs x game.pfs extracted/ -q
        cargo_bin_cmd!("pfs-rs")
            .arg("x")
            .arg(archive.path())
            .arg(extract.path())
            .arg("-q")
            .assert()
            .success();

        // 验证解包后的目录结构:
        // extracted/
        // └── game/          (智能检测被禁用，保留 game/ 目录)
        //     └── system.ini
        extract
            .child("game/system.ini")
            .assert(predicate::path::exists());

        Ok(())
    }

    #[test]
    #[ignore = "Ignored by default because it involves filesystem and process operations"]
    fn test_mixed_files_and_dirs() -> anyhow::Result<()> {
        // 创建临时目录
        let temp = assert_fs::TempDir::new()?;

        // 创建混合内容:
        // dir_a/
        // └── in_dir.txt (content: "in directory")
        // file_b.txt (content: "standalone file")
        let dir_a = temp.child("dir_a");
        dir_a.create_dir_all()?;
        dir_a.child("in_dir.txt").write_str("in directory")?;

        temp.child("file_b.txt").write_str("standalone file")?;

        let archive = temp.child("mixed.pfs");

        // 执行打包命令:
        // pfs-rs c dir_a file_b.txt -o mixed.pfs -q
        cargo_bin_cmd!("pfs-rs")
            .arg("c")
            .arg("dir_a")
            .arg("file_b.txt")
            .arg("-o")
            .arg(archive.path())
            .arg("-q")
            .current_dir(temp.path())
            .assert()
            .success();

        // 创建解包目录
        let extract = temp.child("extracted");
        extract.create_dir_all()?;

        // 执行解包命令:
        // pfs-rs x mixed.pfs extracted/ -q
        cargo_bin_cmd!("pfs-rs")
            .arg("x")
            .arg(archive.path())
            .arg(extract.path())
            .arg("-q")
            .assert()
            .success();

        // 验证解包后的目录结构:
        // extracted/
        // ├── dir_a/
        // │   └── in_dir.txt
        // └── file_b.txt
        extract
            .child("dir_a/in_dir.txt")
            .assert(predicate::path::exists());

        extract
            .child("file_b.txt")
            .assert(predicate::path::exists());

        Ok(())
    }

    #[test]
    #[ignore = "Ignored by default because it involves filesystem and process operations"]
    fn test_list_command() -> anyhow::Result<()> {
        // 创建临时目录
        let temp = assert_fs::TempDir::new()?;

        // 创建测试目录结构:
        // source/
        // └── file1.txt (content: "content")
        let source = temp.child("source");
        source.create_dir_all()?;
        source.child("file1.txt").write_str("content")?;

        let archive = temp.child("test.pfs");

        // 执行打包命令:
        // pfs-rs c source/ -o test.pfs -q
        cargo_bin_cmd!("pfs-rs")
            .arg("c")
            .arg(source.path())
            .arg("-o")
            .arg(archive.path())
            .arg("-q")
            .assert()
            .success();

        // 执行列表命令:
        // pfs-rs l test.pfs
        // 输出验证: 标准输出应包含 "file1.txt"
        cargo_bin_cmd!("pfs-rs")
            .arg("l")
            .arg(archive.path())
            .assert()
            .success()
            .stdout(predicate::str::contains("file1.txt"));

        Ok(())
    }
}
