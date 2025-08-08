use anyhow::{Result, anyhow};
use human_bytes::human_bytes;
use log::{debug, error};
use memmap2::Mmap;
use sha1::{Digest, Sha1};
use std::fmt;
use std::fs::{self, File};
use std::io::{self, Read, Seek, Write};
use std::path::{Path, PathBuf};
use tabled::settings::object::Columns;
use tabled::settings::{Alignment, Style};
use tabled::{Table, Tabled};
use walkdir::WalkDir;

mod util;

//    pf8 structure
//    |magic 'pf8'
//    |index_size 4 //start from index_count (faddr 0x7)
//    |index_count 4
//    |file_entrys[]
//      |name_length 4
//      |name //string with '\0'
//      |00 00 00 00
//      |offset 4
//      |size 4
//    |filesize_count 4
//    |filesize_offsets[] 8 //offset from faddr 0xf, last is 00 00 00 00 00 00 00 00
//    |filesize_count_offset 4 //offset from faddr 0x7

#[allow(dead_code)]
#[derive(Debug)]
struct Pf8Entry {
    name_length: u32,
    name: String,
    // zero: u32,
    offset: u32,
    size: u32,
}

/// Represents a file entry in the PF8 archive
#[derive(Tabled)]
struct Pf8File {
    #[tabled(rename = "File")]
    name: String, // Actually the path in the archive
    #[tabled(rename = "Size", display = "Self::format_size")]
    size: u32,
    // encrypted: bool,
}

impl Pf8File {
    fn format_size(size: &u32) -> String {
        human_bytes(*size)
    }
}
/// Represents a list of files in the PF8 archive
struct Pf8FileList {
    files: Vec<Pf8File>,
}

impl fmt::Display for Pf8FileList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut table = Table::new(&self.files);
        table.with(Style::markdown());
        table.modify(Columns::last(), Alignment::right());

        let count = self.files.len();
        let size = self.files.iter().map(|f| f.size).sum::<u32>();
        let footer = format!("Total: {} files, Total size: {}", count, human_bytes(size));

        write!(f, "{}\n{}", table, footer)
    }
}

fn make_key_pf8_from_bytes(all_bytes: &[u8], index_size: u32) -> Vec<u8> {
    // index_data = from pf8.magic to pf8.filesize_count_offset (start 0x07, length index_size)
    let start = 0x07usize;
    let end = start + index_size as usize;
    let index_data = &all_bytes[start..end];
    let mut hasher = Sha1::new();
    hasher.update(index_data);
    hasher.finalize().to_vec()
}

fn encrypt_pf8(
    buf: &mut [u8],
    start_offset: usize,
    size: usize,
    key: &[u8],
    cover: bool,
) -> Option<Vec<u8>> {
    if cover {
        for i in 0..size {
            buf[start_offset + i] ^= key[i % key.len()];
        }
        None
    } else {
        let mut dst = vec![0; size];
        for i in 0..size {
            dst[i] = buf[start_offset + i] ^ key[i % key.len()];
        }
        Some(dst)
    }
}

fn decrypt_pf8(buf: &[u8], start_offset: usize, size: usize, key: &[u8]) -> Vec<u8> {
    let mut dst = vec![0; size];
    for i in 0..size {
        dst[i] = buf[start_offset + i] ^ key[i % key.len()];
    }
    dst
}

// 只解析 PF8 文件头部分，用于列表功能
fn parse_pf8_header_only(data: &[u8]) -> Result<Vec<Pf8Entry>> {
    if data.len() < 11 {
        return Err(anyhow!("Data too short to parse PF8 header"));
    } // 保证至少能读取到 index_count
    let index_size = u32::from_le_bytes([data[3], data[4], data[5], data[6]]);
    let index_count = u32::from_le_bytes([data[7], data[8], data[9], data[10]]);

    let mut file_entries = Vec::new();
    let mut cur = 0x0B; // 起始位置

    // 计算索引区域的理论结束位置
    // 加上 0x07 是因为PF8格式中，索引数据区从偏移量7开始，总大小为 index_size
    // 但实际解析从 0x0B 开始，所以我们直接用 cur 和这个结束位置比较
    let index_end_pos = (7 + index_size) as usize;

    // 使用 while 循环，条件是当前指针未越过索引区的结尾
    while cur < index_end_pos && cur < data.len() {
        // 检查是否有足够的空间读取 name_length
        if cur + 4 > data.len() {
            break; // 数据不足，无法继续
        }

        let name_length =
            u32::from_le_bytes([data[cur], data[cur + 1], data[cur + 2], data[cur + 3]]);

        cur += 4;

        // 检查是否有足够的空间读取名字、补零和偏移/大小
        if cur + name_length as usize + 12 > data.len() {
            break; // 数据不足
        }

        let name = String::from_utf8(data[cur..cur + name_length as usize].to_vec())?;
        cur += name_length as usize + 4; // 跳过名字和4字节的0

        let offset = u32::from_le_bytes([data[cur], data[cur + 1], data[cur + 2], data[cur + 3]]);
        let size = u32::from_le_bytes([data[cur + 4], data[cur + 5], data[cur + 6], data[cur + 7]]);
        cur += 8;

        file_entries.push(Pf8Entry {
            name_length,
            name,
            offset,
            size,
        });
    }

    // 校验实际解析出的数量是否和文件头中的声明一致
    if file_entries.len() as u32 != index_count {
        error!(
            "Index count mismatch. Expected {}, but found {}. The file may be corrupted or truncated.",
            index_count,
            file_entries.len()
        );
    }

    Ok(file_entries)
}

fn make_pf8_archive(
    basepath: &Path,
    filelist: Vec<(String, u32)>,
    unencrypted_filter: &[&str],
) -> Option<Vec<u8>> {
    let mut data_io = Vec::new();
    let mut fileentry_size = 0;
    let mut filedata_size = 0;
    for (name, size) in &filelist {
        filedata_size += size;
        fileentry_size += name.len() + 16;
    }

    // index_size and index_count should be u32
    let index_count = filelist.len() as u32;
    let index_size = 0x4 + fileentry_size + 0x4 + (index_count as usize + 1) * 0x8 + 0x4;
    let index_size = index_size as u32;

    data_io.extend_from_slice(b"pf8");
    data_io.extend_from_slice(&index_size.to_le_bytes());
    data_io.extend_from_slice(&index_count.to_le_bytes());

    let mut fileoffset = index_size + 0x7;
    let mut filesize_offsets = Vec::new();
    for (name, size) in &filelist {
        let name_bytes = name.as_bytes();
        let name_length = name_bytes.len() as u32;
        data_io.extend_from_slice(&name_length.to_le_bytes());
        data_io.extend_from_slice(name_bytes);
        data_io.extend_from_slice(&[0x0, 0x0, 0x0, 0x0]);
        data_io.extend_from_slice(&fileoffset.to_le_bytes());
        data_io.extend_from_slice(&size.to_le_bytes());
        filesize_offsets.push((data_io.len() - 0x4 - 0xF) as u64);
        fileoffset += size;
    }
    data_io.extend_from_slice(&(index_count + 1).to_le_bytes());
    let filesize_count_offset = (data_io.len() - 0x4 - 0x7) as u32;
    for offset in filesize_offsets {
        data_io.extend_from_slice(&offset.to_le_bytes());
    }
    data_io.extend_from_slice(&[0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0]);
    data_io.extend_from_slice(&filesize_count_offset.to_le_bytes());

    debug!("index_size={}, index_count={}", index_size, index_count);
    debug!(
        "writing index area finished with {} entries!",
        filelist.len()
    );

    for (name, _) in &filelist {
        let pf8_path = util::pf8_filename_str_to_path(name);
        let filepath = basepath.join(pf8_path);
        let mut file = File::open(&filepath).unwrap();
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).unwrap();
        data_io.extend_from_slice(&buffer);
        debug!("copy file {} finished!", filepath.display());
    }

    // Compute key from header bytes directly without cloning/parsing
    let key = make_key_pf8_from_bytes(&data_io, index_size);
    // Encrypt file payloads by iterating original filelist and computing offsets on the fly
    let mut encrypt_offset = (index_size + 0x7) as usize;
    for (name, size) in &filelist {
        let path = name.trim_end_matches('\0');
        let encrypted = !util::search_str_in_vec(unencrypted_filter, path);
        if encrypted {
            encrypt_pf8(&mut data_io, encrypt_offset, *size as usize, &key, true);
            debug!(
                "{} is encrypted at 0x{:X}, size {}",
                path, encrypt_offset, size
            );
        }
        encrypt_offset += *size as usize;
    }
    Some(data_io)
}

/// 将 pf8 文件解包到指定目录
///
/// * `inpath`: artemis pf8 文件路径
/// * `outpath`: 输出目录
/// * `unencrypted_filter`: 未加密的文件后缀列表
pub fn unpack_pf8(inpath: &Path, outpath: &Path, unencrypted_filter: Vec<&str>) -> Result<()> {
    let file = File::open(inpath)?;
    let data = unsafe { Mmap::map(&file)? };

    // 判断是否为 pf8
    let pfs_version = util::get_pfs_version_from_data(&data)?;
    let is_pf8 = pfs_version == 8;

    // Read index_size and build key without copying whole file
    let index_size = u32::from_le_bytes([data[3], data[4], data[5], data[6]]);
    let key = make_key_pf8_from_bytes(&data, index_size);

    // Parse header-only entries for iteration
    let file_entries = parse_pf8_header_only(&data)?;

    for entry in file_entries.iter() {
        let path = entry.name.trim_end_matches('\0');
        let offset = entry.offset as usize;
        let size = entry.size as usize;
        let mut encrypted = true;

        if util::search_str_in_vec(&unencrypted_filter, path) {
            encrypted = false;
        }

        let buf = if encrypted && is_pf8 {
            decrypt_pf8(&data, offset, size, &key)
        } else {
            data[offset..offset + size].to_vec()
        };

        let normalize_path = util::pf8_filename_str_to_path(path);

        let fullpath = outpath.join(normalize_path);
        let basepath = fullpath.parent().unwrap();
        if !basepath.exists() {
            fs::create_dir_all(basepath)?;
        }
        let mut outfile = File::create(fullpath)?;
        outfile.write_all(&buf)?;
        debug!("{}, offset=0x{:X} size={} extracted", path, offset, size);
    }
    Ok(())
}

/// 打包指定目录为 pf8 文件
///
/// * `inpath`: 输入目录
/// * `outpath`: 输出 pf8 文件路径
/// * `unencrypted_filter`: 未加密的文件后缀列表
pub fn pack_pf8(inpath: &Path, outpath: &Path, unencrypted_filter: &[&str]) -> io::Result<()> {
    let mut filelist = Vec::new();
    for entry in WalkDir::new(inpath) {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            let pf8_string = util::path_to_pf8_filename_string(path.strip_prefix(inpath).unwrap());

            let size = fs::metadata(path)?.len() as u32;
            filelist.push((pf8_string, size));
        }
    }
    let data = make_pf8_archive(inpath, filelist, unencrypted_filter).unwrap();
    let mut outfile = File::create(outpath)?;
    outfile.write_all(&data)?;
    Ok(())
}

/// 打包指定多个目录为 pf8 文件
///
/// * `inpath`: 输入目录
/// * `outpath`: 输出 pf8 文件路径
/// * `unencrypted_filter`: 未加密的文件后缀列表
pub fn pack_pf8_multi_input(
    inpath_dirs: &[PathBuf],
    inpath_files: &[PathBuf],
    outpath: &Path,
    unencrypted_filter: &[&str],
) -> io::Result<()> {
    let mut filelist = Vec::new();
    for input in inpath_dirs {
        let prefix = input.parent().unwrap_or(Path::new(""));
        for entry in WalkDir::new(input) {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                let pf8_string =
                    util::path_to_pf8_filename_string(path.strip_prefix(prefix).unwrap());

                let size = fs::metadata(path)?.len() as u32;
                filelist.push((pf8_string, size));
            }
        }
    }

    for input in inpath_files {
        let prefix = input.parent().unwrap_or(Path::new(""));
        for entry in WalkDir::new(input) {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                let pf8_string =
                    util::path_to_pf8_filename_string(path.strip_prefix(prefix).unwrap());

                let size = fs::metadata(path)?.len() as u32;
                filelist.push((pf8_string, size));
            }
        }

        let pf8_string = util::path_to_pf8_filename_string(input.strip_prefix(prefix).unwrap());
        let size = fs::metadata(input)?.len() as u32;
        filelist.push((pf8_string, size));
    }

    let basepath = inpath_dirs[0].parent().unwrap();
    let data = make_pf8_archive(basepath, filelist, unencrypted_filter).unwrap();
    let mut outfile = File::create(outpath)?;
    outfile.write_all(&data)?;
    Ok(())
}

pub fn list_pf8(input: &Path) -> Result<()> {
    let mut file = File::open(input)?;

    // 先读取基本头部信息
    let mut header_buf = vec![0u8; 11];
    file.read_exact(&mut header_buf)?;

    if &header_buf[0..3] != b"pf8" {
        return Err(anyhow!("Not a valid PF8 file"));
    }

    let index_size =
        u32::from_le_bytes([header_buf[3], header_buf[4], header_buf[5], header_buf[6]]);

    // 只读取索引部分，不读取文件数据
    let total_header_size = 7 + index_size as usize;
    let mut full_header = vec![0u8; total_header_size];

    // 重新定位到文件开始
    file.seek(std::io::SeekFrom::Start(0))?;
    file.read_exact(&mut full_header)?;

    // 使用只解析头部的函数
    let file_entries = parse_pf8_header_only(&full_header)?;

    // 构建文件列表
    let file_list = Pf8FileList {
        files: file_entries
            .into_iter()
            .map(|entry| Pf8File {
                name: entry.name,
                size: entry.size,
            })
            .collect(),
    };

    // 打印文件列表表格
    println!("{}", input.display());
    println!();
    println!("{}", file_list);

    Ok(())
}
