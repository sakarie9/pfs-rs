# pfs-rs

English | [中文](./README_CN.md)

Artemis pfs file unpacker and packer written in Rust.

## Description

> Artemis is a game engine. It uses pfs files to store game assets.

This project provides tools to unpack and pack pfs files used by the Artemis system. It is written entirely in Rust.

## Features

- Unpack pfs files
- Pack pfs files

## Usage

```plain
Usage: pfs-rs [OPTIONS] [COMMAND]

Commands:
  unpack  Unpack a Artemis pfs archive
  pack    Pack a directory into a Artemis pfs archive
  list    List contents of a Artemis pfs archive
  help    Print this message or the help of the given subcommand(s)

Options:
  -f, --overwrite  Force overwrite existing files
  -h, --help       Print help
  -V, --version    Print version
```

**Note:** Commands also support short aliases:

- `unpack` can be used as `u`
- `pack` can be used as `p`  
- `list` can be used as `ls`

### Unpack

```plain
Usage: pfs-rs unpack [OPTIONS] <INPUT> <OUTPUT>

Arguments:
  <INPUT>   Input pfs file, can be a glob pattern
  <OUTPUT>  Output directory

Options:
  -s, --split-output  Unpack single file rather than all related files
  -h, --help          Print help (see more with '--help')
```

To unpack a .pfs file:

```bash
pfs-rs unpack <path_to_pfs_file> <path_to_extract_dir>
```

Example:

```plain
└── Artemis
    ├── pfs-rs
    ├── root.pfs
    ├── root.pfs.000
    ├── root.pfs.001
    ├── root.pfs.002
    ├── root.pfs.003
    ├── root.pfs.004
    └── root.pfs.005
```

- To unpack one pfs file

  ```bash
  pfs-rs unpack root.pfs root
  ```

- To unpack all pfs files with glob

  ```bash
  pfs-rs unpack "*.pfs*" .
  ```

  Will unpack all .pfs files to `./root/`.

  > you can also drag pfs files into the executable file to unpack them

### Pack

```plain
Usage: pfs-rs pack <INPUT> <OUTPUT>

Arguments:
  <INPUT>   Input directory
  <OUTPUT>  Output pfs file

Options:
  -h, --help  Print help
```

To pack files into a .pfs file:

```bash
pfs-rs pack <path_to_dir> <path_to_pfs_file>
```

Example:

```plain
├──Artemis
│   ├── font
│   ├── image
│   ├── pc
│   ├── script
│   ├── sound
│   ├── system
│   └── system.ini
├──pfs-rs
```

- To pack whole game folder

  ```bash
  pfs-rs pack Artemis root.pfs
  ```

- To pack multiple folders

  ```plain
  ├── Artemis
  │   ├── font
  │   ├── image
  │   ├── pc
  │   ├── script
  │   ├── sound
  │   ├── system
  │   ├── system.ini
  │   └── pfs-rs
  ```

  ```bash
  pfs-rs font image pc system.ini
  ```

  Will pack specified dirs and files into root.pfs.

  > you can also drag folders into the executable file to pack them

### List

```plain
Usage: pfs-rs list <INPUT>

Arguments:
  <INPUT>  Input pfs file

Options:
  -h, --help  Print help
```

To list contents of a .pfs file:

```bash
pfs-rs list <path_to_pfs_file>
# or use the short alias:
pfs-rs ls <path_to_pfs_file>
```

Example:

```bash
pfs-rs list root.pfs
# or
pfs-rs ls root.pfs
```

This will display a formatted table showing all files in the archive with their sizes and encryption status.

## Related Projects

[pfs-android](https://github.com/sakarie9/pfs-android)

## Acknowledgements

- [YuriSizuku/GalgameReverse](https://github.com/YuriSizuku/GalgameReverse/blob/master/project/artemis/src/artemis_pf8.py)
