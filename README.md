# pfs-rs

English | [中文](./README_CN.md)

Artemis pfs file unpacker and packer written in Rust.

## Description

> [Artemis](https://www.ies-net.com/) is a game engine. It uses pfs files to store game assets.

This project provides tools to unpack and pack pfs files used by the Artemis system. It is written entirely in Rust.

## Features

- Extract pfs files
- Create pfs archives
- List archive contents
- Smart detection of game directories (system.ini)
- rsync-style trailing slash semantics for precise control

## Usage

```plain
Usage: pfs-rs [OPTIONS] [COMMAND]

Commands:
  extract  Extract files from pfs archive(s)
  create   Create pfs archive from files/directories
  list     List contents of pfs archive
  help     Print this message or the help of the given subcommand(s)

Global Options:
  -C, --directory <DIRECTORY>  Change to directory before performing operations
  -f, --force                  Force overwrite existing files
  -q, --quiet                  Quiet mode (no progress output)
  -v, --verbose                Verbose mode (show detailed information)
  -h, --help                   Print help
  -V, --version                Print version
```

**Note:** Commands support aliases:

- `extract` → `x`, `unpack`, `u`
- `create` → `c`, `pack`, `p`  
- `list` → `l`, `ls`

### Extract

```plain
Usage: pfs-rs extract [OPTIONS] <INPUT> [OUTPUT]

Arguments:
  <INPUT>   Input pfs file(s), can be a glob pattern
  [OUTPUT]  Output directory (optional, default: auto-detect)

Options:
  -s, --separate                   Extract each archive to separate directories
      --strip-components <NUMBER>  Strip NUMBER leading components from file names on extraction
  -C, --directory <DIRECTORY>      Change to directory before performing operations
  -f, --force                      Force overwrite existing files
  -q, --quiet                      Quiet mode (no progress output)
  -v, --verbose                    Verbose mode (show detailed information)
  -h, --help                       Print help (see more with '--help')
```

To extract a .pfs file:

```bash
pfs-rs extract <path_to_pfs_file> [output_dir]
# or use alias:
pfs-rs x <path_to_pfs_file> [output_dir]
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

- Extract one pfs file

  ```bash
  pfs-rs extract root.pfs root
  # or just
  pfs-rs x root.pfs
  # Auto-extracts to root/ directory
  ```

- Extract all pfs files with glob pattern

  ```bash
  pfs-rs extract "*.pfs*" .
  ```

  Will extract all .pfs files to `./root/`.

  > You can also drag pfs files onto the executable to extract them

### Create

```plain

Usage: pfs-rs create [OPTIONS] <INPUTS>...

Arguments:
  <INPUTS>...  Input file(s) or directory (supports trailing / for rsync-style behavior)

Options:
  -o, --output <OUTPUT>        Output pfs file (optional, default: root.pfs)
      --no-smart-detect        Disable smart detection (e.g., system.ini auto-pathstrip)
  -C, --directory <DIRECTORY>  Change to directory before performing operations
  -f, --force                  Force overwrite existing files
  -q, --quiet                  Quiet mode (no progress output)
  -v, --verbose                Verbose mode (show detailed information)
  -h, --help                   Print help (see more with '--help')
```

To pack files into a .pfs file:

```bash
pfs-rs create <input_dir_or_file> [-o output.pfs]
# or use alias:
pfs-rs c <input_dir_or_file> [-o output.pfs]
```

#### Example 1: Pack a directory with game structure

```plain
├── Artemis
│   ├── font
│   ├── image
│   ├── pc
│   ├── script
│   ├── sound
│   ├── system
│   └── system.ini
└── pfs-rs
```

```bash
# Pack directory contents
# Always use trailing slash to pack contents only
# Archive contains: font, script, system.ini, etc.
pfs-rs create Artemis/ -o root.pfs

# Pack directory contents (smart detection removes Artemis/)
# Because Artemis/system.ini exists, smart detection removes outer dir
# Archive contains: font, script, system.ini, etc.
pfs-rs create Artemis -o root.pfs

# Pack directory itself (preserves image/ in archive)
# Archive contains: image/img1.png, image/img2.png, etc.
pfs-rs create Artemis/image -o root.pfs.001

# Disable smart detection
# It will not work in artemis because wrong structure
# You should use pfs-rs create Artemis/ -o root.pfs
# Archive contains: Artemis/font, Artemis/script, etc.
pfs-rs create Artemis --no-smart-detect -o badpfs.pfs
```

#### Example 2: Pack multiple directories

```bash
pfs-rs create font image pc script -o game.pfs
# Archive contains: font/, image/, pc/, script/
```

#### Example 3: Default output (root.pfs)

```bash
pfs-rs create Artemis
# Creates: root.pfs
# Archive contains: font/, image/, pc/, script/
# If root.pfs exists:
#   - Without -f: Creates root.pfs.000, root.pfs.001, etc.
#   - With -f: Overwrites root.pfs
```

> You can also drag folders onto the executable to pack them

**rsync-style trailing slash semantics:**

- `dir/` - Packs directory **contents** only
- `dir` - Packs directory **itself** (preserves directory name)

When a single directory contains `system.ini` (classic game structure), smart detection automatically removes the outer directory layer regardless of slash usage.

### List

```plain
Usage: pfs-rs list [OPTIONS] <INPUT>

Arguments:
  <INPUT>  Input pfs file

Options:
  -l, --long                   Show detailed information
  -C, --directory <DIRECTORY>  Change to directory before performing operations
  -f, --force                  Force overwrite existing files
  -q, --quiet                  Quiet mode (no progress output)
  -v, --verbose                Verbose mode (show detailed information)
  -h, --help                   Print help
```

To list contents of a .pfs file:

```bash
pfs-rs list <path_to_pfs_file>
# or use alias:
pfs-rs l <path_to_pfs_file>
```

Example:

```bash
# Simple list
pfs-rs list root.pfs

# Detailed list with sizes
pfs-rs list root.pfs -l
pfs-rs l root.pfs --long
```

## Related Projects

- [pfs-android](https://github.com/sakarie9/pfs-android): An Android app for unpacking Artemis pfs files, based on pf8.

## Acknowledgements

- [YuriSizuku/GalgameReverse](https://github.com/YuriSizuku/GalgameReverse/blob/master/project/artemis/src/artemis_pf8.py)
