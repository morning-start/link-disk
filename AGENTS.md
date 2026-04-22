# link-disk AGENTS.md

## 项目类型：开发工具（CLI工具）

---

## 项目概述

### 项目目标
`link-disk` 是一个 CLI 工具，用于将软件的配置和存储数据从默认位置（通常是 C 盘）转移到其他磁盘分区，通过创建硬链接或软链接的方式，既转移了物理存储，又不影响软件的正常使用。

### 核心功能
- 将文件夹转移到目标位置
- 创建硬链接或软链接回原位置
- 支持多应用、多文件夹的管理
- 通过配置文件灵活配置
- 完整的链接状态管理

### 技术栈

| 组件 | 技术选择 | 版本要求 |
|------|---------|---------|
| 编程语言 | Rust | 2024 edition |
| CLI 框架 | clap | 最新稳定版 |
| 配置格式 | TOML | - |
| 路径解析 | std::path | Rust 标准库 |

### 架构分层

```
┌─────────────────────────────────────────────────────────┐
│                      CLI 层                             │
│            link / unlink / list / status                │
└──────────────────────┬──────────────────────────────────┘
                       │
┌──────────────────────▼──────────────────────────────────┐
│                   Config 层                             │
│              TOML 配置文件解析与管理                      │
└──────────────────────┬──────────────────────────────────┘
                       │
┌──────────────────────▼──────────────────────────────────┐
│                 Workspace 层                            │
│              工作区路径管理与验证                         │
└──────────────────────┬──────────────────────────────────┘
                       │
┌──────────────────────▼──────────────────────────────────┐
│                 Link Ops 层                             │
│           链接创建、删除、验证操作                        │
└──────────────────────┬──────────────────────────────────┘
                       │
┌──────────────────────▼──────────────────────────────────┐
│                  FS Utils 层                            │
│              文件系统底层操作封装                         │
└─────────────────────────────────────────────────────────┘
```

---

## 开发命令

### 环境要求
- Rust 1.75+
- Cargo 包管理器

### 常用命令

```bash
# 开发构建
cargo build                  # Debug 构建
cargo build --release       # Release 构建
cargo run -- [参数]          # 运行程序（调试用）

# 测试
cargo test                   # 运行所有测试
cargo test --release         # Release 模式下测试

# 代码检查
cargo check                  # 快速类型检查
cargo clippy                 # Lint 检查
cargo fmt -- --check         # 格式检查

# 依赖管理
cargo update                 # 更新依赖
cargo tree                   # 查看依赖树

# 清理
cargo clean                  # 清理构建产物
```

### 可执行文件位置
- Debug: `target/debug/link-disk.exe`
- Release: `target/release/link-disk.exe`

---

## 项目结构

```
link-disk/
├── src/
│   ├── main.rs              # 程序入口
│   ├── cli.rs               # CLI 命令解析层（clap）
│   ├── config.rs            # TOML 配置解析 + 路径替换
│   ├── workspace.rs         # 工作区管理器
│   ├── link_ops.rs          # 链接操作（硬链接/软链接）
│   ├── path_resolver.rs     # 路径解析 + 环境变量替换
│   ├── fs_utils.rs          # 文件系统工具
│   └── error.rs             # 统一错误处理
├── docs/
│   ├── architecture.md      # 项目架构文档
│   ├── config.md            # 配置文件说明
│   └── manual.md            # 使用手册
├── tests/
│   └── integration_tests.rs # 集成测试
├── Cargo.toml               # 项目配置
├── config-example.toml      # 配置示例文件
└── AGENTS.md                # 本文件
```

### 核心模块职责

| 模块 | 职责 | 关键类型 |
|------|------|---------|
| `cli.rs` | 命令行参数解析 | `Cli`, `Commands` |
| `config.rs` | TOML 配置加载和验证 | `AppConfig`, `Source` |
| `workspace.rs` | 工作区路径管理 | `Workspace` |
| `link_ops.rs` | 链接创建/删除/验证 | `LinkOp`, `LinkType` |
| `path_resolver.rs` | 环境变量替换 | `PathResolver` |
| `fs_utils.rs` | 文件系统操作封装 | - |
| `error.rs` | 错误类型定义 | `LinkDiskError` |

---

## 代码规范

### 命名约定

| 类型 | 约定 | 示例 |
|------|------|------|
| 模块名 | snake_case | `link_ops`, `fs_utils` |
| 结构体 | PascalCase | `Workspace`, `LinkOp` |
| 枚举变体 | PascalCase | `Symlink`, `Hardlink` |
| 函数 | snake_case | `create_link`, `resolve_path` |
| 变量 | snake_case | `target_path`, `link_type` |
| 常量 | SCREAMING_SNAKE_CASE | `MAX_RETRY_COUNT` |

### 错误处理

```rust
// 使用自定义错误类型
pub enum LinkDiskError {
    IoError(std::io::Error),
    ConfigError(String),
    PathError(String),
    LinkError(String),
}

// 通过 ? 操作符传播
fn example() -> Result<(), LinkDiskError> {
    let config = load_config()?;  // ? 自动转换
    Ok(())
}
```

### 路径处理

**关键规则：**
- Windows 路径使用正斜杠 `/`（Rust 会自动处理）
- 使用 `std::path::Path` 和 `std::path::PathBuf` 而非字符串拼接
- 环境变量替换：`<home>` → 用户目录，`<localappdata>` → 本地应用数据

```rust
// 正确示例
let path = PathBuf::from("<home>/AppData/Roaming");
let resolved = path_resolver.resolve(&path)?;

// 错误示例
let path = "<home>\\AppData\\Roaming".to_string();  // 使用反斜杠
```

### 配置解析

```rust
// 使用 serde 反序列化 TOML
#[derive(Deserialize)]
struct Config {
    workspace: WorkspaceConfig,
    apps: HashMap<String, AppConfig>,
}
```

---

## 测试策略

### 测试框架
- 单元测试：Rust 内置 `#[cfg(test)]`
- 集成测试：`tests/` 目录下的 `.rs` 文件

### 测试运行

```bash
# 运行所有测试（包括单元测试和集成测试）
cargo test

# 运行特定测试
cargo test test_link_creation

# 查看测试覆盖率（需要 tarpaulin）
cargo tarpaulin --out Html
```

### 测试文件组织

```
tests/
└── integration_tests.rs    # 集成测试
    ├── link_operations     # 链接操作测试
    ├── config_parsing     # 配置解析测试
    └── path_resolution     # 路径解析测试
```

### Mock 策略

由于涉及文件系统操作，使用临时目录进行测试：

```rust
#[test]
fn test_link_creation() {
    let temp_dir = TempDir::new().unwrap();
    // 创建测试文件和目录
    // 执行链接操作
    // 验证结果
}
```

---

## 特殊限制

### Windows 平台注意事项

- **硬链接限制**：硬链接不支持跨分区，必须使用软链接
- **管理员权限**：创建符号链接可能需要管理员权限
- **路径格式**：统一使用正斜杠 `/`，Rust 会自动处理

### 配置优先级

1. 命令行参数 `--config <path>` 指定
2. 环境变量 `LINK_DISK_CONFIG`
3. 默认位置 `~/.link-disk/config.toml`

### 日志和调试

```bash
# 启用详细输出
RUST_LOG=debug cargo run -- link --all

# 查看链接状态
link-disk status --verbose
```

---

## 配置文件格式

**配置文件位置：** `~/.link-disk/config.toml`

**基础结构：**

```toml
[workspace]
path = "D:/link-disk-workspace"

[apps.应用名]
name = "显示名称"
on_exists = "skip"  # skip | merge | replace

[[apps.应用名.sources]]
source = "<home>/源路径"
target = "workspace/目标路径"
link_type = "symlink"  # symlink | hardlink
```

### 支持的环境变量占位符

| 占位符 | 说明 | Windows 示例 |
|--------|------|-------------|
| `<home>` | 用户主目录 | `C:\Users\用户名` |
| `<localappdata>` | 本地应用数据 | `C:\Users\用户名\AppData\Local` |
| `<appdata>` | 应用数据目录 | `C:\Users\用户名\AppData\Roaming` |

---

## 常见任务

### 添加新应用支持

1. 在 `config-example.toml` 中添加应用配置示例
2. 在对应模块添加测试用例
3. 运行 `cargo test` 确保测试通过

### 添加新命令

1. 在 `cli.rs` 中定义新的 `clap` 子命令
2. 在 `main.rs` 中添加命令处理逻辑
3. 在对应模块实现业务逻辑
4. 添加集成测试

### 发布新版本

```bash
# 1. 更新版本号
# 编辑 Cargo.toml 中的 version 字段

# 2. 运行完整检查
cargo fmt
cargo clippy
cargo test --release

# 3. 构建发布版本
cargo build --release

# 4. 测试可执行文件
./target/release/link-disk.exe --version
```

---

## 参考资源

- [Rust 官方文档](https://doc.rust-lang.org/)
- [clap 文档](https://docs.rs/clap/)
- [serde TOML](https://docs.rs/serde_yaml/)
- 项目文档：`docs/` 目录
