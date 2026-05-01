# link-disk 架构文档

## 1. 项目概述

**link-disk** 是一个 CLI 工具，用于将软件的配置和存储数据从默认位置（通常是 C 盘）转移到其他磁盘分区，通过创建硬链接或软链接的方式，既转移了物理存储，又不影响软件的正常使用。

### 核心价值

- 将文件夹转移到目标位置
- 创建硬链接或软链接回原位置
- 支持多应用、多文件夹的管理
- 通过配置文件灵活配置
- 完整的链接状态管理

---

## 2. 架构总览

### 2.1 分层架构

```
┌─────────────────────────────────────────────────────────────┐
│                         CLI 层                               │
│                   main.rs + cli.rs                           │
│              职责: 命令解析和业务协调                          │
└──────────────────────────┬──────────────────────────────────┘
                           │
┌──────────────────────────▼──────────────────────────────────┐
│                       Config 层                              │
│                       config.rs                               │
│              职责: TOML 配置解析和数据结构                      │
└──────────────────────────┬──────────────────────────────────┘
                           │
        ┌─────────────────┼─────────────────┐
        ▼                                   ▼
┌───────────────────┐             ┌───────────────────┐
│   Workspace 层     │             │  Path Resolver 层 │
│  workspace.rs      │             │ path_resolver.rs  │
│  职责: 工作区路径   │             │  职责: 占位符替换  │
│      管理          │             └───────────────────┘
└─────────┬─────────┘
          │
          │ 使用
          ▼
┌─────────────────────────────────────────────────────────────┐
│                      Link Ops 层                             │
│                       link_ops.rs                           │
│              职责: 链接操作业务逻辑编排                        │
└──────────────────────────┬──────────────────────────────────┘
                           │
                           │ 调用
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                      FS Utils 层                            │
│                       fs_utils.rs                           │
│              职责: 文件系统原子操作封装                        │
└─────────────────────────────────────────────────────────────┘
```

### 2.2 模块依赖关系

```
main.rs
    ├── cli.rs          (依赖)
    ├── config.rs       (依赖)
    ├── workspace.rs    (依赖)
    ├── link_ops.rs     (依赖)
    │       └── fs_utils.rs
    ├── path_resolver.rs (依赖)
    └── error.rs         (依赖)
```

### 2.3 文件结构

```
src/
├── main.rs              # 程序入口，命令调度
├── cli.rs               # CLI 命令定义 (clap)
├── config.rs            # 配置文件解析 (TOML)
├── workspace.rs         # 工作区管理
├── link_ops.rs          # 链接操作逻辑
├── path_resolver.rs     # 路径占位符解析
├── fs_utils.rs          # 文件系统工具
└── error.rs             # 错误类型定义
```

---

## 3. 核心模块详解

### 3.1 CLI 层 (main.rs + cli.rs)

**文件位置:** `src/main.rs`, `src/cli.rs`

**职责:**
- 程序入口点
- 命令行参数解析 (clap)
- 业务逻辑协调
- 错误处理和用户提示

**支持的命令:**

| 命令 | 说明 | 关键选项 |
|------|------|---------|
| `init` | 初始化工作区和配置文件 | `--path`, `--force` |
| `link` | 创建链接（转移文件夹并创建链接） | `--all`, `--dry-run`, `--force`, `-v` |
| `unlink` | 移除链接并恢复原文件位置 | `--all`, `--force`, `--keep-files` |
| `list` | 列出所有已配置的应用和链接 | `--app` |
| `status` | 检查链接状态是否正常 | [应用名...] |
| `repair` | 修复损坏的链接 | [应用名...], `--force` |

### 3.2 Config 层 (config.rs)

**文件位置:** `src/config.rs`

**职责:**
- TOML 配置文件解析
- 配置数据结构定义
- 应用配置验证

**关键类型:**

```rust
pub struct Config {
    pub workspace: Workspace,
    pub apps: HashMap<String, AppConfig>,
}

pub struct Workspace {
    pub path: PathBuf,
}

pub struct AppConfig {
    pub name: String,
    pub enabled: bool,
    pub on_exists: Option<String>,
    pub sources: Vec<Source>,
}

pub struct Source {
    pub source: String,
    pub target: String,
    pub link_type: String,
    pub _source_type: String,
}
```

### 3.3 Workspace 层 (workspace.rs)

**文件位置:** `src/workspace.rs`

**职责:**
- 工作区目录初始化
- 配置文件路径管理
- 目标路径解析
- 用户目录路径展开 (~ 前缀)

**关键函数:**

| 函数 | 说明 |
|------|------|
| `init(path)` | 初始化工作区目录和配置文件 |
| `config_dir()` | 获取配置目录路径 (`~/.link-disk`) |
| `config_path()` | 获取配置文件路径 (`~/.link-disk/config.toml`) |
| `expand_path(path)` | 展开 ~ 为用户主目录 |
| `resolve_target(workspace, relative)` | 解析目标路径 |

### 3.4 Path Resolver 层 (path_resolver.rs)

**文件位置:** `src/path_resolver.rs`

**职责:**
- 环境变量占位符替换
- 路径展开和规范化

**支持的占位符:**

| 占位符 | 说明 | Windows 示例 |
|--------|------|-------------|
| `<home>` | 用户主目录 | `C:\Users\<用户名>` |
| `<appdata>` | AppData/Roaming | `...\AppData\Roaming` |
| `<localappdata>` | AppData/Local | `...\AppData\Local` |
| `<documents>` | 文档文件夹 | `...\Documents` |
| `<desktop>` | 桌面 | `...\Desktop` |
| `<downloads>` | 下载文件夹 | `...\Downloads` |
| `<temp>` | 临时文件夹 | `...\AppData\Local\Temp` |
| `<programfiles>` | Program Files | `C:\Program Files` |
| `<programfilesx86>` | Program Files (x86) | `C:\Program Files (x86)` |

**关键函数:**

```rust
pub struct PathResolver;

impl PathResolver {
    pub fn expand(path: &str) -> String                    // 展开所有占位符
    pub fn resolve_if_exists(path: &str) -> Option<PathBuf> // 展开并检查是否存在
}
```

### 3.5 FS Utils 层 (fs_utils.rs)

**文件位置:** `src/fs_utils.rs`

**职责:**
- 文件系统原子操作封装
- 跨平台文件系统操作抽象
- 符号链接安全删除（区分目录/文件符号链接）

**关键函数:**

| 函数 | 说明 |
|------|------|
| `copy_dir_recursive(src, dst)` | 递归复制目录（含文件） |
| `move_dir_cross_filesystem(src, dst)` | 跨分区移动（复制后删除源） |
| `normalize_path(path)` | 规范化路径（统一正斜杠，小写） |
| `ensure_parent_exists(path)` | 确保父目录存在 |
| `remove_if_exists(path, verbose)` | 安全删除（自动判断文件/目录/符号链接） |
| `rename(src, dst)` | 重命名/移动文件 |
| `create_symlink(target, link)` | 创建符号链接（自动区分文件/目录） |
| `read_link(path)` | 读取链接目标 |
| `hard_link(target, link)` | 创建硬链接 |

**设计原则:**
- 每个函数都是原子操作
- 无业务逻辑，仅文件系统操作
- 统一的错误处理 (anyhow::Result)
- Windows 符号链接特殊处理（先尝试 remove_dir，失败则 remove_file）

### 3.6 Link Ops 层 (link_ops.rs)

**文件位置:** `src/link_ops.rs`

**职责:**
- 链接操作的业务逻辑编排
- 状态检查和决策
- 调用 fs_utils 执行实际操作

**关键类型:**

```rust
pub enum LinkType {
    Symlink,   // 符号链接（软链接）
    Hardlink,  // 硬链接
}

pub enum OnExists {
    Skip,      // 跳过 - 目标已存在时不操作
    Merge,     // 合并 - 合并源到目标后删除源，继续创建链接
    Overwrite, // 覆盖 - 删除源后继续创建链接
    Replace,   // 替换 - 删除目标，移动源到目标，创建链接
}

pub struct LinkRequest {
    pub source: PathBuf,
    pub target: PathBuf,
    pub link_type: LinkType,
    pub on_exists: OnExists,
    pub force: bool,
}

impl LinkOps {
    pub fn link(request: &LinkRequest, verbose: bool) -> Result<()>
    pub fn unlink(source, target, keep_files, verbose) -> Result<()>
    pub fn check_status(source, target) -> &'static str
}
```

**链接状态返回值:**

| 状态 | 说明 |
|------|------|
| `"linked"` | 链接正常，目标和源都存在 |
| `"broken"` | 链接损坏（目标不存在） |
| `"both_exist"` | 源和目标都存在 |
| `"source_only"` | 只有源存在 |
| `"target_only"` | 只有目标存在 |
| `"none"` | 都不存在 |

---

## 4. 业务流程

业务流程、使用场景和状态流转图已移至独立的 [业务流程文档](workflows.md)。

该文档包含：
- link 命令主流程
- 符号链接检查流程 (force 逻辑)
- on_exists 策略处理流程
- unlink 命令执行流程
- repair 命令执行流程
- 链接状态流转图
- 使用示例和故障排除

---

## 5. 错误处理

### 5.1 错误类型

```rust
// error.rs - 自定义错误类型（预留）
pub enum LinkDiskError {
    Io(std::io::Error),
    Config(String),
    Path(String),
    Link(String),
}

pub type Result<T> = std::result::Result<T, LinkDiskError>;
```

### 5.2 当前实现

使用 `anyhow::Result` 统一错误处理：
- 通过 `.with_context()` 为底层 IO 错误添加可读的操作描述
- 通过 `.bail!()` 或 `anyhow::anyhow!()` 创建上下文丰富的错误
- 在 `main()` 中统一捕获并输出错误信息

### 5.3 错误传播链

```
fs_utils (anyhow::Result)
    │ .with_context() 添加 IO 操作上下文
    └── ? 向上传播
           │
link_ops (anyhow::Result)
    │ .with_context() 添加链接操作上下文
    └── ? 向上传播
           │
main.rs (anyhow::Result)
    │ .bail!() / .with_context()
    └── ? 向上传播
           │
main() → eprintln!("Error: {}", e); std::process::exit(1)
```

---

## 6. 命名规范

| 类型 | 规范 | 示例 |
|------|------|------|
| 模块名 | snake_case | `link_ops`, `fs_utils` |
| 结构体 | PascalCase | `Workspace`, `LinkOps` |
| 枚举变体 | PascalCase | `Symlink`, `Hardlink` |
| 函数 | snake_case | `create_link`, `resolve_path` |
| 变量 | snake_case | `target_path`, `link_type` |
| 字段 | snake_case | `workspace_path` |

---

## 7. 跨平台考虑

### 7.1 Windows 特殊处理

- **符号链接创建**: 使用 `symlink_dir` / `symlink_file`
- **符号链接删除**: 先尝试 `remove_dir`（目录符号链接），失败则尝试 `remove_file`（文件符号链接）
- **权限**: 需要管理员权限或开发者模式
- **路径格式**: 内部使用反斜杠 `\`

### 7.2 Unix 系统

- 符号链接通过 `symlink` 统一创建
- 符号链接通过 `remove_file` 统一删除
- 硬链接支持同文件系统

### 7.3 路径规范化

路径比较时统一使用正斜杠 `/` 并转小写：

```rust
pub fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace("\\", "/").to_lowercase()
}
```

---

## 8. 设计原则

### 8.1 SOLID 原则

| 原则 | 应用 |
|------|------|
| **单一职责** | 每个模块职责清晰: Config 解析配置, Workspace 管路径, FsUtils 原子操作 |
| **开放封闭** | LinkType, OnExists 枚举易于扩展新变体 |
| **里氏替换** | 公共函数接受 `&Path` 而非 `&PathBuf`，更通用 |
| **接口隔离** | FsUtils 提供多个小而专注的原子方法 |
| **依赖倒置** | LinkOps 编排 FsUtils，main 协调 LinkOps |

### 8.2 分层原则

```
上层调用下层，下层不调用上层
业务逻辑层 (LinkOps) 调用工具层 (FsUtils)
协调层 (main) 调用业务层 (LinkOps)
```

---

## 9. 测试策略

### 9.1 单元测试

| 模块 | 测试内容 |
|------|---------|
| `path_resolver.rs` | 占位符展开测试 |

### 9.2 测试运行

```bash
# 运行所有测试
cargo test

# 运行特定测试
cargo test test_home_placeholder
```

---

## 10. 扩展指南

### 10.1 添加新命令

1. 在 `cli.rs` 定义子命令枚举变体
2. 在 `main.rs` 的 `run()` 函数中添加处理逻辑
3. 如需新操作，在 `link_ops.rs` 或 `fs_utils.rs` 实现

### 10.2 添加新占位符

在 `path_resolver.rs` 的 `replace_placeholders` 函数中添加:

```rust
if let Some(path) = dirs::new_dir_function() && result.contains("<new>") {
    result = result.replace("<new>", &path.to_string_lossy());
}
```

### 10.3 添加新链接策略

在 `link_ops.rs` 的 `OnExists` 枚举中添加新变体：
1. 更新 `from_str` 匹配
2. 在 `link` 方法的 `match request.on_exists` 分支中实现相应逻辑

### 10.4 当前依赖

| 依赖 | 用途 |
|------|------|
| `clap` | CLI 参数解析 |
| `toml` | 配置文件解析 |
| `serde` | 序列化/反序列化 |
| `anyhow` | 错误处理 |
| `dirs` | 系统目录路径获取 |
| `spinners` | 终端进度指示器 |
