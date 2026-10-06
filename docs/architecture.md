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

### 2.1 三层架构

采用经典的**三层架构**（commands → domain → infra），依赖方向单向向下，
上层可依赖下层，下层绝不反向依赖上层：

```
┌─────────────────────────────────────────────────────────────┐
│                    commands 层 (CLI 调度)                    │
│   cli.rs: Cli/CliContext + 各子命令 *Args 参数结构体          │
│   commands/mod.rs: Command trait + dispatch() 统一调度       │
│   load_config() 解析优先级；select_apps() 统一应用选择        │
│   init / link / unlink / list / status / repair             │
└──────────────────────────┬──────────────────────────────────┘
                           │ 依赖
┌──────────────────────────▼──────────────────────────────────┐
│                    domain 层 (核心业务逻辑)                   │
│   link_ops: 链接状态机 (LinkOps / LinkRequest / LinkType)    │
│   strategies: on_exists 策略 (OnExists / OnExistsAction)    │
│   request_builder: 配置 → 领域请求 (build_link_request)      │
│   file_mover: 目录合并 / 文件移回    link_status: 状态枚举    │
└──────────────────────────┬──────────────────────────────────┘
                           │ 依赖
┌──────────────────────────▼──────────────────────────────────┐
│                    infra 层 (基础设施)                        │
│   config: TOML 解析与校验 (Config / AppConfig / Source)      │
│   fs_utils: FileSystem trait + FsUtils 实现                  │
│   path_resolver: 占位符注册表与展开    workspace: 工作区管理  │
└─────────────────────────────────────────────────────────────┘
```

**分层约束**：

- infra 不依赖 domain（链接请求构建曾放错在 infra，已迁至 `domain/request_builder.rs`）
- domain 不感知 CLI 选项：过程信息用 `tracing::debug!`，`--verbose` 只存在于 commands 层
- 配置值域校验通过 serde 强类型枚举前移到反序列化阶段

### 2.2 模块依赖关系

```
main.rs ──► cli.rs ──► commands/mod.rs ──► commands/*
                              │
                ┌─────────────┴──────────────┐
                ▼                            ▼
          domain/*  ──────────────►  infra/*
   (link_ops, strategies,          (config, fs_utils,
    request_builder, file_mover,    path_resolver,
    link_status)                    workspace)
```

### 2.3 文件结构

```
src/
├── main.rs                # 程序入口、日志初始化（RUST_LOG > --verbose）
├── lib.rs                 # 公共库导出（供集成测试使用）
├── cli.rs                 # Cli / CliContext / Commands / 各 *Args（clap）
├── commands/
│   ├── mod.rs             # Command trait、dispatch()、load_config()、select_apps()
│   ├── init.rs            # 初始化工作区与配置
│   ├── link.rs            # 创建链接（转移 + 建链）
│   ├── unlink.rs          # 移除链接（可选移回文件）
│   ├── list.rs            # 列出应用配置
│   ├── status.rs          # 检查链接状态
│   └── repair.rs          # 修复损坏链接
├── domain/
│   ├── mod.rs             # 领域层导出
│   ├── link_ops.rs        # 链接状态机 + 内存版测试替身（#[cfg(test)]）
│   ├── link_status.rs     # LinkStatus 枚举与检查器
│   ├── file_mover.rs      # merge_dirs / move_back
│   ├── request_builder.rs # resolve_source_target / build_link_request
│   └── strategies.rs      # OnExists 策略（skip/merge/preserve/replace）
└── infra/
    ├── mod.rs             # 基础设施层导出
    ├── config.rs          # Config / AppConfig / Source + validate()
    ├── fs_utils.rs        # FileSystem trait / FsUtils / detect_symlink_cycle
    ├── path_resolver.rs   # 占位符注册表 + PathResolver
    └── workspace.rs       # Workspace（工作区初始化、配置路径、目标解析）
```

---

## 3. 核心模块详解

### 3.1 CLI 层 (cli.rs + commands/)

**职责:** 参数解析、全局选项传递、命令调度、用户交互输出。

- `Cli`（Parser）持有全局选项 `verbose` / `config` 与子命令；`Cli::context()` 把
  全局选项提取为 `CliContext`，命令实现只需 `execute(&self, ctx)`，**不再 match
  整个 `Cli`**，也不存在 `unreachable!()`。
- 每个子命令的参数是独立的 `clap::Args` 结构体（`LinkArgs`、`InitArgs` 等），
  `Commands` 枚举以元组变体持有；对应的 `*Args` 实现 `Command` trait。

**支持的命令:**

| 命令 | 说明 | 关键选项 |
|------|------|---------|
| `init` | 初始化工作区和配置文件 | `--path`, `--force`（force 会用模板重建配置文件） |
| `link` | 创建链接（转移文件夹并创建链接） | `[apps]`, `--all`, `--dry-run`, `--force`, `-v` |
| `unlink` | 移除链接并恢复原文件位置 | `[apps]`, `--all`, `--force`, `--keep-files` |
| `list` | 列出所有已配置的应用和链接 | `--app`（未知应用报错） |
| `status` | 检查链接状态是否正常 | `[apps]`（未知应用报错） |
| `repair` | 修复损坏的链接 | `[apps]`, `--all`, `--force` |

**配置解析优先级**（`commands/mod.rs::load_config`）：

1. `--config <path>` 命令行参数
2. `LINK_DISK_CONFIG` 环境变量
3. 默认位置 `~/.link-disk/config.toml`

**应用选择**（`select_apps`）：`--all` 或应用列表为空 → 所有启用的应用
（`BTreeMap` 保证按 ID 字典序输出稳定）；显式指定不存在的应用 → 统一报错。

### 3.2 Config 层 (infra/config.rs)

**职责:** TOML 解析、配置数据结构、静态校验。

```rust
pub struct Config {
    pub workspace: Workspace,                    // { path: PathBuf }
    pub apps: BTreeMap<String, AppConfig>,       // 按应用 ID 稳定排序
    pub custom_placeholders: HashMap<String, String>,
}

pub struct AppConfig {
    pub name: String,
    pub enabled: bool,
    pub on_exists: Option<OnExists>,             // 强类型枚举，非法值在反序列化即报错
    pub sources: Vec<Source>,
}

pub struct Source {
    pub source: String,                          // 支持占位符
    pub target: String,
    pub link_type: LinkType,                     // 默认 symlink
    pub on_exists: Option<OnExists>,             // 源级覆盖，优先级高于应用级
}
```

**`validate()` 校验项**（值域校验已由 serde 完成，这里只做结构关系校验）：

| 校验 | 目的 |
|------|------|
| 工作区路径非空、应用名非空 | 基本完整性 |
| 占位符合法性（`check_placeholders`） | 以注册表为唯一事实来源 |
| name/target 工作区路径安全性（`check_workspace_relative`） | 拦截 `..` 逃逸、绝对路径、Windows 非法字符，避免运行期在意外位置写数据 |
| 同应用 target 去重（`check_target_conflicts`） | 防止多个 source 争抢同一工作区目录 |

> 注：应用级与源级 `on_exists` 是**覆盖关系**（源级 > 应用级 > 默认 skip，见
> [config.md](config.md)），不存在需要拒绝的"冲突组合"。历史版本曾以
> `check_strategy_conflicts` 拒绝这些合法配置，与文档承诺相矛盾，已移除。

### 3.3 Workspace 层 (infra/workspace.rs)

**职责:** 工作区目录初始化、配置文件管理、目标路径解析。

| 函数 | 说明 |
|------|------|
| `init(path, force)` | 初始化工作区；`force` 时用模板重建 `~/.link-disk/config.toml` |
| `init_with_template(path, force, template)` | `init` 的通用实现，`{}` 为工作区路径占位符 |
| `config_dir()` / `config_path()` | `~/.link-disk` / `~/.link-disk/config.toml` |
| `resolve_target(workspace, relative)` | `workspace.join(relative)`（不手动替换分隔符） |

### 3.4 Path Resolver 层 (infra/path_resolver.rs)

**职责:** 占位符替换与路径展开。注册表模式（`RwLock<HashMap>` + `LazyLock`），
符合 OCP：新增占位符只需注册条目；支持运行时 `register_placeholder()` 注册
自定义占位符（内置占位符不可被覆盖）。

| 占位符 | 说明 | Windows 示例 |
|--------|------|-------------|
| `<home>` | 用户主目录 | `C:\Users\<用户名>` |
| `<appdata>` | AppData/Roaming | `...\AppData\Roaming` |
| `<localappdata>` | AppData/Local | `...\AppData\Local` |
| `<documents>` | 文档文件夹 | `...\Documents` |
| `<desktop>` | 桌面 | `...\Desktop` |
| `<downloads>` | 下载文件夹 | `...\Downloads` |
| `<temp>` | 系统临时目录（`std::env::temp_dir()`） | `...\AppData\Local\Temp` |
| `<programfiles>` | Program Files | `C:\Program Files` |
| `<programfilesx86>` | Program Files (x86) | `C:\Program Files (x86)` |

| 函数 | 说明 |
|------|------|
| `expand(path)` | 展开所有已注册占位符（先扫描输入中的 `<...>`，仅命中项查表） |
| `expand_home(path)` | 展开 `~` 前缀 |
| `is_known_placeholder(p)` | 注册表存在性检查（配置校验共用） |

### 3.5 FS Utils 层 (infra/fs_utils.rs)

**职责:** 文件系统操作的**单一抽象点**。`FileSystem` trait 同时包含状态查询
（`exists` / `is_symlink` / `is_dir`）与变更操作，`FsUtils` 提供真实实现。
领域层一律经由 trait 访问文件，从而：

1. 跨平台差异（符号链接创建/删除）集中在一处；
2. 测试可用内存替身驱动链接状态机（见 3.6），不依赖符号链接权限。

| trait 方法 | 说明 |
|------|------|
| `exists / is_symlink / is_dir` | 状态查询 |
| `normalize_path(path)` | 统一分隔符；Windows 转小写用于比较 |
| `read_link(path)` | 读取符号链接原始目标 |
| `create_dir_all(path)` | 创建目录及缺失父目录 |
| `copy_dir_recursive(src, dst)` | BFS 迭代复制（避免深目录栈溢出） |
| `move_path(src, dst)` | 同卷优先 `rename`，失败回退复制+删除（跨卷） |
| `ensure_parent_exists(path)` | 确保父目录存在 |
| `remove_if_exists(path)` | 安全删除（Windows 区分目录/文件符号链接） |
| `rename / create_symlink / hard_link` | 原子操作封装 |

独立辅助函数 `detect_symlink_cycle(path)`：沿符号链接链追踪（上限
`MAX_SYMLINK_DEPTH = 64`），创建软链前拒绝会造成回环的目标。

**设计原则:** 每个方法都是原子操作；无业务逻辑；统一 `anyhow::Result` 错误上下文。

### 3.6 Link Ops 层 (domain/link_ops.rs)

**职责:** 链接操作的状态机编排。

```rust
pub enum LinkType { Symlink, Hardlink }

pub struct LinkRequest {
    pub source: PathBuf,
    pub target: PathBuf,
    pub link_type: LinkType,
    pub on_exists: OnExists,
    pub force: bool,
}

impl LinkOps {
    pub fn link_with_fs(request: &LinkRequest, fs: &dyn FileSystem) -> Result<()>;
    pub fn unlink_with_fs(source: &Path, target: &Path, keep_files: bool, fs: &dyn FileSystem) -> Result<()>;
    pub fn check_status(source: &Path, target: &Path) -> LinkStatus;
}
```

**link 状态机** —— 所有分支先归一到"source 不存在 + target 存在"，再统一建链：

| 当前状态 | 动作 |
|---------|------|
| source 是指向 target 的链接 | 幂等，直接返回 |
| source 是指向其他位置的链接 | `force`：删除后重建；否则报错 |
| source 真实存在 + target 不存在 | 移动 source → target |
| source 真实存在 + target 存在 | 按 `on_exists` 策略归一（skip 报错） |
| source 不存在 + target 不存在 | 创建空 target 目录 |
| source 不存在 + target 存在 | 直接建链 |

`domain/link_ops.rs` 的 `#[cfg(test)]` 模块提供内存版 `MemoryFs`，
上表每个分支都有无权限依赖的单元测试覆盖。

### 3.7 Strategies 层 (domain/strategies.rs)

`OnExists::execute(source, target, fs)` 返回 `OnExistsAction`，主流程据此决定
后续动作——策略只负责"冲突归一"，移动/建链由 `LinkOps` 统一处理：

| 策略 | 行为 | 返回 |
|------|------|------|
| `skip`（默认） | 不动任何数据 | `Skip` → 主流程报错中断 |
| `replace` | 删除 target | `ContinueWithMove` → 移动 source |
| `merge` | 合并 source 到 target（同名保留 target）后删除 source | `ContinueWithoutMove` |
| `preserve`（别名 `overwrite`） | 删除 source，保留 target 数据 | `ContinueWithoutMove` |

### 3.8 Link Status 层 (domain/link_status.rs)

`LinkStatusChecker::check(source, target)` 基于 `is_symlink` / `exists`
分类六种状态；对外统一经由 `LinkOps::check_status` 使用。

| 状态 | 说明 |
|------|------|
| `Linked` | source 是链接且 target 存在 |
| `Broken` | source 是链接但 target 已丢失 |
| `BothExist` | 双方都存在（source 非链接） |
| `SourceOnly` / `TargetOnly` / `None` | 其余组合 |

---

## 4. 业务流程

业务流程、使用场景和状态流转图见 [workflows.md](workflows.md)：

- link / unlink / repair 命令主流程
- on_exists 策略处理流程
- 链接状态流转图与故障排除

---

## 5. 错误处理

统一使用 `anyhow::Result`，无自定义错误类型：

- infra 层：`.with_context(|| ...)` 为底层 IO 添加操作描述
- domain 层：`anyhow::bail!` 表达业务规则违例（如 "pointing to different target"）
- commands 层：`e.context(format!("Failed to link {app}:{path}"))` 补充应用/路径定位
- `main()`：`eprintln!("Error: {e:#}")` 之前统一捕获并以退出码 1 结束

日志：`RUST_LOG` 环境变量优先；未设置时按 `--verbose` 推导 `link_disk=debug|info`。
领域层过程细节全部 `debug!` 级，用户反馈由 commands 层 `println!` 承担。

---

## 6. 跨平台考虑

### Windows

- 符号链接创建用 `symlink_dir` / `symlink_file`（需要管理员或开发者模式）
- 符号链接删除：先 `remove_dir`，失败再 `remove_file`
- `normalize_path` 额外转小写用于链接目标比较

### Unix

- 符号链接创建/删除用 `symlink` / `remove_file` 统一处理
- 硬链接不支持目录、不支持跨分区

### 路径处理

内部统一使用 `std::path::Path`/`PathBuf`，不手动替换分隔符；
仅模板渲染时把工作区路径转为正斜杠写入 TOML。

---

## 7. 测试策略

| 层级 | 位置 | 说明 |
|------|------|------|
| 领域状态机单测 | `domain/link_ops.rs #[cfg(test)]` | `MemoryFs` 内存替身，覆盖 link/unlink 全分支，**无需符号链接权限** |
| 策略/合并单测 | `domain/strategies.rs`、`infra/fs_utils.rs` | 真实临时目录（tempfile），不依赖链接权限 |
| 配置校验单测 | `infra/config.rs` | 占位符、路径安全、target 冲突、serde 值域 |
| 集成测试 | `tests/integration_tests.rs` | 仅使用公共 API；涉及创建符号链接的用 `#[cfg_attr(windows, ignore)]` 标记，特权环境跑 `cargo test -- --ignored` |

```bash
cargo test                    # 全部（Windows 无特权时跳过链接创建类）
cargo test -- --ignored      # 补充真实符号链接链路
cargo clippy --all-targets   # lint
```

---

## 8. 扩展指南

### 8.1 添加新命令

1. `cli.rs`：新增 `*Args` 结构体（derive `clap::Args`）与 `Commands` 元组变体
2. `commands/`：新模块为该 `*Args` 实现 `Command` trait
3. `commands/mod.rs::dispatch()`：加 match 分支
4. 业务逻辑进 `domain/`，基础设施进 `infra/`；添加集成测试

### 8.2 添加新占位符

在 `infra/path_resolver.rs` 中：`placeholders` 模块加常量 → 注册表 `LazyLock`
初始化里 `insert` 一个 resolver 闭包。校验与展开自动生效。

### 8.3 添加新 on_exists 策略

在 `domain/strategies.rs`：`OnExists` 枚举加变体（serde 自动接受小写名）→
`execute()` 加 match 分支返回 `OnExistsAction` → `link_ops` 主流程无需改动。

---

## 9. 当前依赖

| 依赖 | 用途 |
|------|------|
| `clap` | CLI 参数解析 |
| `toml` / `serde` | 配置文件解析与强类型反序列化 |
| `anyhow` | 错误处理与上下文链 |
| `dirs` | 系统目录路径获取 |
| `tracing` / `tracing-subscriber` | 分级日志 |
| `spinners` | 终端进度指示器（仅非 verbose 模式） |
