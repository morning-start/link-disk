# link-disk

将软件的配置和存储数据从默认位置（通常是 C 盘）转移到其他磁盘分区，通过创建硬链接或软链接的方式，既转移了物理存储，又不影响软件的正常使用。

## 特性

- **跨分区转移** - 将数据从 C 盘转移到其他分区，释放磁盘空间
- **透明访问** - 通过链接机制，软件仍然能正常访问转移后的数据
- **多应用支持** - 通过配置文件管理多个应用的链接配置
- **状态监控** - 检查链接状态，修复损坏的链接
- **平台支持** - 支持 Windows (硬链接/软链接) 和 Unix 系统

## 安装

### 从源码构建

```bash
git clone https://github.com/your-username/link-disk.git
cd link-disk
cargo build --release
```

可执行文件位于 `target/release/link-disk.exe` (Windows) 或 `target/release/link-disk` (Unix)。

### 环境要求

- Rust 1.75+
- Windows: 需要管理员权限创建符号链接

## 快速开始

### 1. 初始化

```bash
# 使用默认工作区路径 (D:/link-disk-workspace)
link-disk init

# 指定工作区路径
link-disk init --path "E:/my-workspace"
```

初始化后会创建默认配置文件 `~/.link-disk/config.toml`。

### 2. 配置应用

编辑配置文件 `~/.link-disk/config.toml`:

```toml
[workspace]
path = "D:/link-disk-workspace"

[apps.vscode]
name = "VSCode"
on_exists = "skip"

[[apps.vscode.sources]]
source = "<home>/AppData/Roaming/Code"
target = "vscode/Roaming"
link_type = "symlink"

[apps.chrome]
name = "Chrome"
on_exists = "skip"

[[apps.chrome.sources]]
source = "<home>/AppData/Local/Google/Chrome"
target = "chrome/Local"
link_type = "symlink"
```

### 3. 创建链接

```bash
# 链接所有已配置的应用
link-disk link --all

# 链接指定应用
link-disk link vscode

# 模拟运行（不实际执行）
link-disk link vscode --dry-run

# 强制重新链接（删除已存在的链接）
link-disk link vscode --force
```

### 4. 其他命令

```bash
# 列出所有已配置的应用和链接
link-disk list

# 检查链接状态
link-disk status

# 检查指定应用的链接状态
link-disk status vscode

# 修复损坏的链接
link-disk repair --force

# 取消链接并恢复原文件位置
link-disk unlink vscode --force
```

## 命令详解

### init - 初始化

```bash
link-disk init [OPTIONS]

OPTIONS:
  -p, --path <PATH>    工作区路径
  -f, --force          强制重新初始化
```

### link - 创建链接

```bash
link-disk link [APPS]... [OPTIONS]

OPTIONS:
  -a, --all            处理所有已配置的应用
  -d, --dry-run        模拟运行，不实际执行操作
  -f, --force          强制处理（删除已存在的软链接后重新链接）
  -v, --verbose        详细输出

EXAMPLES:
  link-disk link vscode chrome        # 链接 vscode 和 chrome
  link-disk link --all                # 链接所有应用
  link-disk link --all --dry-run      # 模拟链接所有应用
```

### unlink - 移除链接

```bash
link-disk unlink [APPS]... [OPTIONS]

OPTIONS:
  -a, --all            处理所有已配置的应用
  -f, --force          强制执行，不确认
  -k, --keep-files     只删除链接，不移动文件

EXAMPLES:
  link-disk unlink vscode             # 移除 vscode 的链接并恢复文件位置
  link-disk unlink --all --force       # 移除所有链接
```

### list - 列出应用

```bash
link-disk list [OPTIONS]

OPTIONS:
  -a, --app <APP>     只显示指定应用的链接
```

### status - 检查状态

```bash
link-disk status [APPS]...

EXAMPLES:
  link-disk status               # 检查所有应用的状态
  link-disk status vscode        # 检查 vscode 的状态
```

### repair - 修复链接

```bash
link-disk repair [APPS]... [OPTIONS]

OPTIONS:
  -f, --force          强制修复（自动创建缺失的链接）
```

## 配置说明

### 占位符

配置文件中的路径可以使用以下占位符：

| 占位符 | 说明 | Windows 示例 |
|--------|------|-------------|
| `<home>` | 用户主目录 | `C:\Users\用户名` |
| `<appdata>` | 应用数据目录 | `C:\Users\用户名\AppData\Roaming` |
| `<localappdata>` | 本地应用数据 | `C:\Users\用户名\AppData\Local` |
| `<documents>` | 文档目录 | `C:\Users\用户名\Documents` |
| `<desktop>` | 桌面目录 | `C:\Users\用户名\Desktop` |
| `<downloads>` | 下载目录 | `C:\Users\用户名\Downloads` |
| `<temp>` | 临时目录 | `C:\Users\用户名\AppData\Local\Temp` |

### on_exists 策略

| 值 | 说明 |
|----|------|
| `skip` | 如果目标已存在，跳过操作 |
| `replace` | 删除已存在的目标后重新创建 |
| `merge` | 合并目录内容 |
| `overwrite` | 覆盖源文件 |

### link_type 链接类型

| 值 | 说明 | 跨分区 |
|----|------|--------|
| `symlink` | 符号链接（软链接） | 支持 |
| `hardlink` | 硬链接 | 不支持（仅同分区） |

## 工作原理

```
┌─────────────────────────────────────────────────────────┐
│                    初始状态                              │
│  源路径: C:\Users\<user>\AppData\Local\Code            │
└─────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────┐
│              link 命令执行后                             │
│                                                         │
│  1. 将源路径的文件/文件夹移动到工作区                      │
│     C:\Users\<user>\AppData\Local\Code                  │
│     → D:\link-disk-workspace\vscode\Local\Code          │
│                                                         │
│  2. 在原位置创建符号链接指向工作区                         │
│     C:\Users\<user>\AppData\Local\Code                  │
│     → 符号链接 → D:\link-disk-workspace\vscode\Local\Code│
└─────────────────────────────────────────────────────────┘
```

软件仍然访问原路径，但实际数据存储在工作区。

## 项目结构

```
link-disk/
├── src/
│   ├── main.rs              # 程序入口
│   ├── cli.rs               # CLI 命令解析 (clap)
│   ├── config.rs            # TOML 配置解析
│   ├── workspace.rs         # 工作区管理
│   ├── link_ops.rs          # 链接操作
│   ├── path_resolver.rs     # 路径解析
│   ├── fs_utils.rs          # 文件系统工具
│   └── error.rs             # 错误处理
├── Cargo.toml
├── config-example.toml      # 配置示例
├── README.md
└── AGENTS.md               # 开发文档
```

## 开发

```bash
# 构建
cargo build
cargo build --release

# 测试
cargo test

# 代码检查
cargo clippy
cargo fmt -- --check

# 运行
cargo run -- link --all -v
```

## 注意事项

### Windows

- 创建符号链接需要管理员权限或启用开发者模式
- 硬链接不支持跨分区
- 开发者模式：设置 → 隐私和安全性 → 开发者选项 → 启用开发者模式

### 安全考虑

- 链接指向的目录被删除后，链接会损坏（断开的链接）
- 使用 `link-disk status` 定期检查链接状态
- 使用 `link-disk repair` 修复损坏的链接

## License

MIT
