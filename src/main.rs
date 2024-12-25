use clap::{arg, command, ArgGroup};
use memmap2::Mmap;
use regex::Regex;
use sha1::{Digest, Sha1};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug)]
struct Pf8Entry {
    name_length: u32,
    name: String,
    // zero: u32,
    offset: u32,
    size: u32,
}

#[derive(Debug)]
struct Pf8 {
    magic: [u8; 3],
    index_size: u32,
    index_count: u32,
    file_entries: Vec<Pf8Entry>,
    file_count: u32,
    filesize_offsets: Vec<u64>,
    filesize_count_offset: u32,
    data: Vec<u8>,
}

fn make_key_pf8(pf8: &Pf8) -> Vec<u8> {
    // index_data = from pf8.magic to pf8.filesize_count_offset
    let index_data = &pf8.data[0x07..(0x07 + pf8.index_size as usize)];
    // let index_data = [];
    // println!("{:?}", index_data);
    // println!("{:?}", &pf8.filesize_count_offset);

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

fn parse_pf8(data: Vec<u8>) -> Option<Pf8> {
    if &data[0..3] != b"pf8" {
        println!("Error: invalid pf8 file!");
        return None;
    }

    let index_size = u32::from_le_bytes([data[3], data[4], data[5], data[6]]);
    let index_count = u32::from_le_bytes([data[7], data[8], data[9], data[10]]);
    let mut pf8 = Pf8 {
        data,
        magic: *b"pf8",
        index_size,
        index_count,
        file_entries: Vec::new(),
        file_count: 0,
        filesize_offsets: Vec::new(),
        filesize_count_offset: 0,
    };

    let mut cur = 0x0B;
    for _ in 0..index_count {
        let name_length = u32::from_le_bytes([
            pf8.data[cur],
            pf8.data[cur + 1],
            pf8.data[cur + 2],
            pf8.data[cur + 3],
        ]);
        let name =
            String::from_utf8(pf8.data[cur + 4..cur + 4 + name_length as usize].to_vec()).unwrap();
        cur += name_length as usize + 8;
        let offset = u32::from_le_bytes([
            pf8.data[cur],
            pf8.data[cur + 1],
            pf8.data[cur + 2],
            pf8.data[cur + 3],
        ]);
        let size = u32::from_le_bytes([
            pf8.data[cur + 4],
            pf8.data[cur + 5],
            pf8.data[cur + 6],
            pf8.data[cur + 7],
        ]);
        pf8.file_entries.push(Pf8Entry {
            name_length,
            name,
            offset,
            size,
        });
        cur += 8;
    }

    pf8.file_count = u32::from_le_bytes([
        pf8.data[cur],
        pf8.data[cur + 1],
        pf8.data[cur + 2],
        pf8.data[cur + 3],
    ]);
    cur += 4;
    for _ in 0..pf8.file_count {
        let filesize_offset = u64::from_le_bytes([
            pf8.data[cur],
            pf8.data[cur + 1],
            pf8.data[cur + 2],
            pf8.data[cur + 3],
            pf8.data[cur + 4],
            pf8.data[cur + 5],
            pf8.data[cur + 6],
            pf8.data[cur + 7],
        ]);
        pf8.filesize_offsets.push(filesize_offset);
        cur += 8;
    }
    pf8.filesize_count_offset = u32::from_le_bytes([
        pf8.data[cur],
        pf8.data[cur + 1],
        pf8.data[cur + 2],
        pf8.data[cur + 3],
    ]);
    Some(pf8)
}

fn make_pf8_archive(
    basepath: &Path,
    filelist: Vec<(String, u32)>,
    unencrypted_filter: Vec<&str>,
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

    println!("tail={}, index_size={}", data_io.len(), index_size);
    println!(
        "writing index area finished with {} entries!",
        filelist.len()
    );

    for (name, _) in &filelist {
        let pf8_path = pf8_filename_str_to_path(name);
        let filepath = basepath.join(pf8_path);
        let mut file = File::open(&filepath).unwrap();
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).unwrap();
        data_io.extend_from_slice(&buffer);
        println!("copy file {} finished!", filepath.display());
    }

    let data = data_io.clone();
    let pf8 = parse_pf8(data).unwrap();
    let key = make_key_pf8(&pf8);
    // println!("sha1 hash key is {}", hex::encode(&key));
    let count = pf8.index_count as usize;
    let file_entries = pf8.file_entries;
    let re_unencrypted: Vec<Regex> = unencrypted_filter
        .iter()
        .map(|&t| Regex::new(t).unwrap())
        .collect();
    for i in 0..count {
        let path = file_entries[i].name.trim_end_matches('\0');
        let offset = file_entries[i].offset as usize;
        let size = file_entries[i].size as usize;
        let mut encrypted = true;
        for t in &re_unencrypted {
            if t.is_match(path) {
                encrypted = false;
                break;
            }
        }
        if encrypted {
            encrypt_pf8(&mut data_io, offset, size, &key, true);
            println!("{} is encrypted at 0x{:X}, size {}", path, offset, size);
        }
    }
    Some(data_io)
}

fn unpack_pf8(
    inpath: &Path,
    outpath: &Path,
    unencrypted_filter: Vec<&str>,
    pathlist: Option<Vec<String>>,
) -> io::Result<()> {
    let file = File::open(inpath)?;
    let data = unsafe { Mmap::map(&file)? };
    let pf8 = parse_pf8(data.to_vec()).unwrap();
    let key = make_key_pf8(&pf8);
    let count = pf8.index_count as usize;
    let file_entries = pf8.file_entries;

    let re_unencrypted: Vec<Regex> = unencrypted_filter
        .iter()
        .map(|&t| Regex::new(t).unwrap())
        .collect();
    for i in 0..count {
        let path = file_entries[i].name.trim_end_matches('\0');
        if let Some(ref paths) = pathlist {
            if !paths.contains(&path.to_string()) {
                println!("skipped! {}", path);
                continue;
            }
        }
        let offset = file_entries[i].offset as usize;
        let size = file_entries[i].size as usize;
        let mut encrypted = true;
        for t in &re_unencrypted {
            if t.is_match(path) {
                encrypted = false;
                break;
            }
        }

        let buf = if encrypted {
            decrypt_pf8(&data, offset, size, &key)
        } else {
            data[offset..offset + size].to_vec()
        };

        let normalize_path = pf8_filename_str_to_path(path);

        let fullpath = outpath.join(normalize_path);
        let basepath = fullpath.parent().unwrap();
        if !basepath.exists() {
            fs::create_dir_all(basepath)?;
        }
        let mut outfile = File::create(fullpath)?;
        outfile.write_all(&buf)?;
        println!("{}, offset=0x{:X} size={} extracted", path, offset, size);
    }
    Ok(())
}

fn pack_pf8(inpath: &Path, outpath: &Path, unencrypted_filter: Vec<&str>) -> io::Result<()> {
    let mut filelist = Vec::new();
    for entry in WalkDir::new(inpath) {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            let pf8_string = path_to_pf8_filename_string(path.strip_prefix(inpath).unwrap());

            let size = fs::metadata(path)?.len() as u32;
            filelist.push((pf8_string, size));
        }
    }
    let data = make_pf8_archive(inpath, filelist, unencrypted_filter).unwrap();
    let mut outfile = File::create(outpath)?;
    outfile.write_all(&data)?;
    Ok(())
}

/// 将反斜杠分隔的字符串转换为 PathBuf
fn pf8_filename_str_to_path(s: &str) -> PathBuf {
    s.split("\\").collect()
}

/// 将 Path 转换为反斜杠分隔的字符串
fn path_to_pf8_filename_string(path: &Path) -> String {
    // 将每个组件都转换为 &str 并收集到 Vec 中
    let components: Vec<&str> = path
        .iter()
        .map(|os_str| os_str.to_str().unwrap_or(""))
        .collect();
    // 用反斜杠拼接生成字符串
    components.join("\\")
}

fn main() {
    let matches = command!()
        .about("Pack and unpack tool for artemis .pf8 archive")
        .version("0.2")
        .author("devseed")
        .arg(arg!(-u --unpack <FILE> "Unpack a .pf8 file").required(false))
        .arg(arg!(-p --pack <DIR> "Pack a directory into a .pf8 file").required(false))
        .arg(arg!(-o --outpath <DIR> "Output directory or file path").default_value("out"))
        .group(
            ArgGroup::new("operation")
                .args(["unpack", "pack"])
                .required(true),
        )
        .get_matches();

    if let Some(inpath) = matches.get_one::<String>("unpack") {
        let outpath = matches.get_one::<String>("outpath").unwrap();
        unpack_pf8(
            &PathBuf::from(inpath),
            &PathBuf::from(outpath),
            vec![r"\.mp4$", r"\.flv$"],
            None,
        )
        .unwrap();
    } else if let Some(inpath) = matches.get_one::<String>("pack") {
        let outpath = matches.get_one::<String>("outpath").unwrap();
        pack_pf8(
            &PathBuf::from(inpath),
            &PathBuf::from(outpath),
            vec![r"\.mp4$", r"\.flv$"],
        )
        .unwrap();
    } else {
        println!("pf8tool argument error!");
    }
}
