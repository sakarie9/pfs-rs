# pfs-rs

[English](./README.md) | 中文

一个用 Rust 编写的 Artemis pfs 文件解包器和打包器。

## 描述

> [Artemis](https://www.ies-net.com/) 是一个游戏引擎。它使用 pfs 文件来存储游戏资源。

该项目提供了解包和打包 Artemis 系统使用的 pfs 文件的工具。完全用 Rust 编写。

## 功能

- 解包 pfs 文件
- 创建 pfs 压缩包
- 列出压缩包内容
- 游戏目录的智能检测（system.ini）
- rsync 风格的尾部斜杠语义以实现精确控制

## 使用方法

```plain
使用方法: pfs-rs [OPTIONS] [COMMAND]

命令:
  extract  从 pfs 压缩包中解包文件
  create   从文件/目录创建 pfs 压缩包
  list     列出 pfs 压缩包的内容
  help     打印此消息或给定子命令的帮助

全局选项:
  -C, --directory <DIRECTORY>  切换到指定目录后执行操作
  -f, --force                  强制覆盖现有文件
  -q, --quiet                  安静模式（无进度输出）
  -v, --verbose                详细模式（显示详细信息）
  -h, --help                   打印帮助
  -V, --version                打印版本
```

**注意：** 命令支持别名：

- `extract` → `x`, `unpack`, `u`
- `create` → `c`, `pack`, `p`  
- `list` → `l`, `ls`

### 解包

```plain
使用方法: pfs-rs extract [OPTIONS] <INPUT> [OUTPUT]

参数:
  <INPUT>   输入 pfs 文件，可以是通配符模式
  [OUTPUT]  输出目录（可选，默认：自动检测）

选项:
  -s, --separate                   将每个压缩包解包到单独的目录
      --strip-components <NUMBER>  解包时从文件名中删除 NUMBER 个前导组件
  -C, --directory <DIRECTORY>      切换到指定目录后执行操作
  -f, --force                      强制覆盖现有文件
  -q, --quiet                      安静模式（无进度输出）
  -v, --verbose                    详细模式（显示详细信息）
  -h, --help                       打印帮助（使用 '--help' 查看更多）
```

解包 .pfs 文件：

```bash
pfs-rs extract <pfs_文件路径> [输出目录]
# 或使用别名:
pfs-rs x <pfs_文件路径> [输出目录]
```

示例：

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

- 解包单个 pfs 文件

  ```bash
  pfs-rs extract root.pfs root
  # 或直接使用
  pfs-rs x root.pfs
  # 自动解包到 root/ 目录
  ```

- 使用通配符模式解包所有 pfs 文件

  ```bash
  pfs-rs extract "*.pfs*" .
  ```

  将所有 .pfs 文件解包到 `./root/`。

  > 你也可以将 pfs 文件拖到执行文件上来解包它们

### 打包

```plain

使用方法: pfs-rs create [OPTIONS] <INPUTS>...

参数:
  <INPUTS>...  输入文件或目录（支持尾部 / 以实现 rsync 风格行为）

选项:
  -o, --output <OUTPUT>        输出 pfs 文件（可选，默认：root.pfs）
      --no-smart-detect        禁用智能检测（如 system.ini 自动路径剥离）
  -C, --directory <DIRECTORY>  切换到指定目录后执行操作
  -f, --force                  强制覆盖现有文件
  -q, --quiet                  安静模式（无进度输出）
  -v, --verbose                详细模式（显示详细信息）
  -h, --help                   打印帮助（使用 '--help' 查看更多）
```

将文件打包为 .pfs 文件：

```bash
pfs-rs create <输入目录或文件> [-o 输出.pfs]
# 或使用别名:
pfs-rs c <输入目录或文件> [-o 输出.pfs]
```

#### 示例 1：打包具有游戏结构的目录

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
# 打包目录内容
# 使用尾部斜杠仅打包内容
# 压缩包包含：font, script, system.ini 等
pfs-rs create Artemis/ -o root.pfs

# 打包目录内容（智能检测删除 Artemis/）
# 因为 Artemis/system.ini 存在，智能检测会删除外层目录
# 压缩包包含：font, script, system.ini 等
pfs-rs create Artemis -o root.pfs

# 打包目录本身（保留 image/ 在压缩包中）
# 压缩包包含：image/img1.png, image/img2.png 等
pfs-rs create Artemis/image -o root.pfs.001

# 禁用智能检测
# 在 artemis 中不会生效因为结构不对
# 你应该使用 pfs-rs create Artemis/ -o root.pfs
# 压缩包包含：Artemis/font, Artemis/script 等
pfs-rs create Artemis --no-smart-detect -o badpfs.pfs
```

#### 示例 2：打包多个目录

```bash
pfs-rs create font image pc script -o game.pfs
# 压缩包包含：font/, image/, pc/, script/
```

#### 示例 3：默认输出（root.pfs）

```bash
pfs-rs create Artemis
# 创建：root.pfs
# 压缩包包含：font/, image/, pc/, script/
# 如果 root.pfs 存在：
#   - 未使用 -f：创建 root.pfs.000, root.pfs.001 等
#   - 使用 -f：覆盖 root.pfs
```

> 你也可以将文件夹拖到执行文件上来打包它们

**rsync 风格的尾部斜杠语义：**

- `dir/` - 仅打包目录**内容**
- `dir` - 打包目录**本身**（保留目录名）

当单个目录包含 `system.ini`（经典游戏结构）时，智能检测会自动删除外层目录，无论是否使用斜杠。

### 列表

```plain
使用方法: pfs-rs list [OPTIONS] <INPUT>

参数:
  <INPUT>  输入 pfs 文件

选项:
  -l, --long                   显示详细信息
  -C, --directory <DIRECTORY>  切换到指定目录后执行操作
  -f, --force                  强制覆盖现有文件
  -q, --quiet                  安静模式（无进度输出）
  -v, --verbose                详细模式（显示详细信息）
  -h, --help                   打印帮助
```

列出 .pfs 文件的内容：

```bash
pfs-rs list <pfs_文件路径>
# 或使用别名:
pfs-rs l <pfs_文件路径>
```

示例：

```bash
# 简单列表
pfs-rs list root.pfs

# 带有大小的详细列表
pfs-rs list root.pfs -l
pfs-rs l root.pfs --long
```

## 相关项目

- [pfs-android](https://github.com/sakarie9/pfs-android)：一个用于解包 Artemis pfs 文件的 Android 应用，基于 pf8。

## 致谢

- [YuriSizuku/GalgameReverse](https://github.com/YuriSizuku/GalgameReverse/blob/master/project/artemis/src/artemis_pf8.py)
