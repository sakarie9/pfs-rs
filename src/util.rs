use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Result};

pub fn is_file_pf8(path: &Path) -> Result<bool> {
    // 打开文件
    let mut file = File::open(path)?;

    // 创建一个缓冲区来存储前三个字节
    let mut buffer = [0; 3];

    // 读取文件的前三个字节
    file.read_exact(&mut buffer)?;

    // 将字节缓冲区转换为字符串
    let header = std::str::from_utf8(&buffer).expect("Invalid UTF-8 sequence");

    // 判断是否为字符串 "pf8"
    if header == "pf8" {
        Ok(true)
    } else {
        Err(anyhow!("The file is not a pf8 file, found: {:?}", header))
    }
}

/// 将反斜杠分隔的字符串转换为 PathBuf
pub fn pf8_filename_str_to_path(s: &str) -> PathBuf {
    s.split("\\").collect()
}

/// 将 Path 转换为反斜杠分隔的字符串
pub fn path_to_pf8_filename_string(path: &Path) -> String {
    // 将每个组件都转换为 &str 并收集到 Vec 中
    let components: Vec<&str> = path
        .iter()
        .map(|os_str| os_str.to_str().unwrap_or(""))
        .collect();
    // 用反斜杠拼接生成字符串
    components.join("\\")
}

pub fn get_str_extension(s: &str) -> Option<&str> {
    let path = Path::new(s);
    path.extension().and_then(|s| s.to_str())
}

pub fn search_str_in_vec(vec: &[&str], s: &str) -> bool {
    vec.iter().any(|x| *x == s)
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
        return Err(anyhow!("Invalid file name"));
    }
    Err(anyhow!("Failed to get file name"))
}

/// 根据输入路径，返回匹配到的文件路径列表
/// - 若输入为目录，则返回目录下后缀为 .pfs 或 .pfs.xxx 的文件路径
/// - 若输入为文件，则返回同目录下与该文件文件名一致（前缀相同）的所有相关文件路径
pub fn find_pfs_files(input: &Path) -> Result<Vec<PathBuf>> {
    let mut results = Vec::new();

    if input.is_dir() {
        // 如果是目录，则查找所有 .pfs 或 .pfs.xxx 文件
        for entry in fs::read_dir(input)? {
            let path = entry?.path();
            if path.is_file() {
                if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                    // 简单判断：文件名以 ".pfs" 结尾，或形如 ".pfs.xxx"
                    // 例如：NUKITASHI.pfs / NUKITASHI.pfs.000
                    if name.ends_with(".pfs") || is_pfs_xxx(name) {
                        results.push(path);
                    }
                }
            }
        }
    } else if input.is_file() {
        results.push(input.to_path_buf());
    }

    results.sort();

    Ok(results)
}

/// 判断文件名是否形如 *.pfs.xxx，例如 "NUKITASHI.pfs.000"
pub fn is_pfs_xxx(name: &str) -> bool {
    // 若文件名含有 ".pfs." 并且后面还有其他字符，则判定为符合
    // 例如：NUKITASHI.pfs.000
    if let Some(pos) = name.find(".pfs.") {
        if name.len() > pos + 5 {
            return true;
        }
    }
    false
}
