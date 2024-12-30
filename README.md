# pfs_rs

Artemis .pfs file unpacker and packer written in Rust.

## Description

> Artemis is a game engine. It uses .pfs files to store game assets.

This project provides tools to unpack and pack .pfs files used by the Artemis system. It is written entirely in Rust.

## Features

- Unpack .pfs files
- Pack .pfs files

## Usage

```plain
Usage: pfs_rs [OPTIONS] [INPUTS]... [COMMAND]

Commands:
  unpack  Unpack a Artemis pfs archive
  pack    Pack a directory into a Artemis pfs archive
  help    Print this message or the help of the given subcommand(s)

Options:
  -f, --overwrite  Force overwrite existing files
  -h, --help       Print help
  -V, --version    Print version
```

### Unpack

```plain
Usage: pfs_rs unpack [OPTIONS] <INPUT> <OUTPUT>

Arguments:
  <INPUT>   Input file, ending in .pfs, can be a glob pattern
  <OUTPUT>  Output directory

Options:
  -s, --split-output  Unpack single file rather than all related files
  -h, --help          Print help (see more with '--help')
```

To unpack a .pfs file:

```bash
pfs_rs unpack <path_to_pfs_file> <path_to_extract_dir>
```

Example:

```plain
└── Artemis
    ├── pfs_rs
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
  pfs_rs unpack root.pfs root
  ```

- To unpack all pfs files with glob

  ```bash
  pfs_rs unpack "*.pfs*" .
  ```

  Will unpack all .pfs files to `./root/`.

  > you can also drag pfs files into the executable file to unpack them

### Pack

```plain
Usage: pfs_rs pack <INPUT> <OUTPUT>

Arguments:
  <INPUT>   Input directory
  <OUTPUT>  Output file, ending in .pfs

Options:
  -h, --help  Print help
```

To pack files into a .pfs file:

```bash
pfs_rs pack <path_to_dir> <path_to_pfs_file>
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
├──pfs_rs
```

- To pack whole game folder

  ```bash
  pfs_rs pack Artemis root.pfs
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
  │   └── pfs_rs
  ```

  ```bash
  pfs_rs font image pc system.ini
  ```

  Will pack specified dirs and files into root.pfs.

  > you can also drag folders into the executable file to pack them

## Acknowledgements

- [YuriSizuku/GalgameReverse](https://github.com/YuriSizuku/GalgameReverse/blob/master/project/artemis/src/artemis_pf8.py)
