use anyhow::{Result, anyhow};
use std::path::{Path, PathBuf};

pub fn get_pfs_version_from_data(data: &[u8]) -> Result<usize> {
    // 将字节缓冲区转换为字符串
    let header = std::str::from_utf8(&data[0..3]).map_err(|_| anyhow!("Invalid input file!"))?;

    // 判断是否为字符串 "pf8"
    if header == "pf8" {
        Ok(8)
    } else if header == "pf6" {
        Ok(6)
    } else {
        Err(anyhow!(
            "The file is not a pf8 or pf6 file, found: {:?}",
            header
        ))
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

pub fn search_str_in_vec(vec: &[&str], s: &str) -> bool {
    vec.iter().any(|x| *x == s)
}
