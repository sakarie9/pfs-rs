# pfs_rs

[English](./README.md) | 中文

一个用 Rust 编写的 Artemis .pfs 文件解包器和打包器。

## 描述

> Artemis 是一个游戏引擎。它使用 .pfs 文件来存储游戏资源。

本项目提供了用于解包和打包 Artemis 系统使用的 .pfs 文件的工具。它完全用 Rust 编写。

## 功能

- 解包 .pfs 文件
- 打包 .pfs 文件

## 用法

```plain
用法：pfs_rs [选项] [命令]

命令：
  unpack  解包 Artemis pfs 存档
  pack    将目录打包成 Artemis pfs 存档
  help    打印此消息或给定子命令的帮助信息

选项：
  -f, --overwrite  强制覆盖现有文件
  -h, --help       打印帮助信息
  -V, --version    打印版本信息
```

### 解包

```plain
用法：pfs_rs unpack [选项] <输入> <输出>

参数：
  <输入>   输入文件，以 .pfs 结尾，可以是 glob 模式
  <输出>  输出目录

选项：
  -s, --split-output  解包单个文件而不是所有相关文件
  -h, --help          打印帮助信息 (使用 '--help' 查看更多)
```

解包 .pfs 文件：

```bash
pfs_rs unpack <pfs文件路径> <解压目录路径>
```

示例：

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

- 解包一个 pfs 文件

  ```bash
  pfs_rs unpack root.pfs root
  ```

- 使用 glob 模式解包所有 pfs 文件

  ```bash
  pfs_rs unpack "*.pfs*" .
  ```

  会将所有 .pfs 文件解包到 `./root/` 目录。

  > 你也可以将 pfs 文件拖拽到可执行文件上来解包它们

### 打包

```plain
用法：pfs_rs pack <输入> <输出>

参数：
  <输入>   输入目录
  <输出>  输出文件，以 .pfs 结尾

选项：
  -h, --help  打印帮助信息
```

将文件打包成 .pfs 文件：

```bash
pfs_rs pack <目录路径> <pfs文件路径>
```

示例：

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

- 打包整个游戏文件夹

  ```bash
  pfs_rs pack Artemis root.pfs
  ```

- 打包多个文件夹

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

  会将指定的目录和文件打包到 root.pfs。

  > 你也可以将文件夹拖拽到可执行文件上来打包它们

## 致谢

- [YuriSizuku/GalgameReverse](https://github.com/YuriSizuku/GalgameReverse/blob/master/project/artemis/src/artemis_pf8.py)
