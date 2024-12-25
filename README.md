# pfs_rs

Artemis .pfs file unpacker and packer written in Rust.

## Description

> Artemis is a game engine. It uses .pfs files to store game assets.

This project provides tools to unpack and pack .pfs files used by the Artemis system. It is written entirely in Rust.

## Features

- Unpack .pfs files
- Pack .pfs files

## Usage

To unpack a .pfs file:

```bash
pfs_rs unpack <path_to_pfs_file> <path_to_extract_dir>
```

To pack files into a .pfs file:

```bash
pfs_rs pack <path_to_dir> <path_to_pfs_file>
```

## Acknowledgements

- [YuriSizuku/GalgameReverse](https://github.com/YuriSizuku/GalgameReverse/blob/master/project/artemis/src/artemis_pf8.py)
