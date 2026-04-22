# link-disk 项目架构文档

## 1. 项目概述

### 1.1 项目目标

`link-disk` 是一个 CLI 工具，用于将软件的配置和存储数据从默认位置（通常是 C 盘）转移到其他磁盘分区（D 盘、E 盘等），通过创建硬链接或软链接的方式，既转移了物理存储，又不影响软件的正常使用。

### 1.2 核心功能

- 将文件夹转移到目标位置
- 创建硬链接或软链接回原位置
- 支持多应用、多文件夹的管理
- 通过配置文件灵活配置
- 完整的链接状态管理

---

## 2. 技术选型

### 2.1 技术栈

| 组件 | 技术选择 | 理由 |
|------|---------|------|
| 编程语言 | Rust | 性能好、跨平台、错误处理优秀 |
| CLI 框架 | clap | Rust 生态最成熟的 CLI 库 |
| 配置格式 | TOML | 专为配置文件设计，支持注释 |
| 路径解析 | std::path | Rust 标准库 |

### 2.2 支持的链接类型

| 类型 | 说明 | 跨分区支持 |
|------|------|-----------|
| 硬链接 | 同一文件系统内共享 inode | ❌ 不支持 |
| 软链接 | 符号链接，类似快捷方式 | ✅ 支持 |

> **注意**: Windows 上硬链接有诸多限制，建议默认使用软链接。

---

## 3. 目录结构

```
link-disk/
├── src/
│   ├── main.rs              # 程序入口
│   ├── cli.rs               # CLI 命令解析层
│   ├── config.rs            # TOML 配置解析 + 路径替换
│   ├── workspace.rs         # 工作区管理器
│   ├── link_ops.rs          # 链接操作（硬链接/软链接）
│   ├── path_resolver.rs      # 路径解析 + 环境变量替换
│   ├── fs_utils.rs          # 文件系统工具
│   └── error.rs             # 统一错误处理
├── docs/
│   ├── architecture.md      # 项目架构文档
│   ├── config.md            # 配置文件说明
│   └── manual.md            # 使用手册
├── tests/
│   └── integration_tests.rs # 集成测试
├── Cargo.toml
├── config-example.toml      # 配置示例文件
└── README.md
```

---

## 4. 模块架构

### 4.1 架构分层图

```
┌─────────────────────────────────────────────────────────┐
│                      CLI 层                             │
│            link / unlink / list / status                │
│                 init / repair / migrate                 │
└──────────────────────┬──────────────────────────────────┘
                       │
┌──────────────────────▼──────────────────────────────────┐
│                   Config 层                             │
│              TOML 配置文件解析与管理                      │
│            支持 <home> 等环境变量替换                      │
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
│         (移动、复制、目录创建、权限检测)                   │
└─────────────────────────────────────────────────────────┘
```

### 4.2 核心模块说明

#### 4.2.1 CLI 层 (cli.rs)

负责解析用户命令行输入，定义所有可用命令。

**命令列表:**

| 命令 | 说明 |
|------|------|
| `init` | 初始化工作区，创建配置目录 |
| `link` | 创建链接（转移文件+创建链接） |
| `unlink` | 移除链接（删除链接+恢复文件） |
| `list` | 列出所有已管理的链接 |
| `status` | 检查链接状态（是否有效） |
| `repair` | 修复损坏的链接 |
| `migrate` | 迁移配置到新位置 |

#### 4.2.2 Config 层 (config.rs)

负责配置文件的解析和管理。

**配置结构:**

```toml
[workspace]
path = "D:/link-disk-workspace"

[apps.应用名]
name = "应用显示名"
on_exists = "skip"   # skip | merge | replace

[[apps.应用名.sources]]
source = "<home>/源路径"
target = "目标子路径"
link_type = "symlink"  # symlink | hardlink
```

#### 4.2.3 Path Resolver 层 (path_resolver.rs)

负责路径中的环境变量替换。

**支持的占位符:**

| 占位符 | 说明 | Windows 示例 |
|--------|------|--------------|
| `<home>` | 用户主目录 | `C:/Users/YourName` |
| `<appdata>` | AppData(Roaming) | `C:/Users/YourName/AppData/Roaming` |
| `<localappdata>` | AppData/Local | `C:/Users/YourName/AppData/Local` |
| `<documents>` | 文档文件夹 | `C:/Users/YourName/Documents` |
| `<desktop>` | 桌面 | `C:/Users/YourName/Desktop` |
| `<downloads>` | 下载文件夹 | `C:/Users/YourName/Downloads` |
| `<temp>` | 临时文件夹 | `C:/Users/YourName/AppData/Local/Temp` |
| `<programfiles>` | Program Files | `C:/Program Files` |
| `<programfilesx86>` | Program Files (x86) | `C:/Program Files (x86)` |

#### 4.2.4 Workspace 层 (workspace.rs)

负责工作区的管理和验证。

**职责:**
- 验证工作区路径是否存在
- 计算实际的目标路径
- 管理工作区元数据

#### 4.2.5 Link Ops 层 (link_ops.rs)

负责链接的创建、删除和验证。

**操作流程 (link 命令):**

```
1. 解析源路径（替换环境变量）
2. 解析目标路径
3. 检查源路径是否存在
4. 检查目标路径是否存在
5. 根据 on_exists 策略处理
6. 移动源文件到目标位置
7. 在源位置创建链接指向目标
8. 验证链接是否有效
```

#### 4.2.6 FS Utils 层 (fs_utils.rs)

文件系统底层操作封装。

**功能:**
- 目录创建（递归创建）
- 文件移动
- 目录复制
- 链接创建
- 路径存在性检测
- 权限检测

---

## 5. 配置结构

### 5.1 完整配置示例

```toml
# link-disk.toml - 工作区配置文件

[workspace]
path = "D:/link-disk-workspace"

# ============ 应用分组 ============
[apps.vscode]
name = "VSCode"
on_exists = "skip"

[[apps.vscode.sources]]
source = "<home>/AppData/Roaming/Code"
target = "vscode/Roaming"
link_type = "symlink"

[[apps.vscode.sources]]
source = "<home>/.code"
target = "vscode/config"
link_type = "symlink"

[apps.chrome]
name = "Chrome"
on_exists = "skip"

[[apps.chrome.sources]]
source = "<home>/AppData/Local/Google/Chrome"
target = "chrome/Local"
link_type = "symlink"
```

### 5.2 on_exists 策略

| 策略 | 行为 |
|------|------|
| `skip` | 如果目标文件夹已存在，跳过此链接（默认） |
| `merge` | 如果目标已存在，合并源内容到目标 |
| `replace` | 如果目标已存在，先删除目标，再创建链接 |

---

## 6. 数据流

### 6.1 Link 操作数据流

```
用户执行: link-disk link vscode
                    │
                    ▼
┌─────────────────────────────────────┐
│         1. CLI 解析命令              │
│    确定要对 vscode 这个 app 操作      │
└─────────────────┬───────────────────┘
                  │
                  ▼
┌─────────────────────────────────────┐
│         2. 加载配置文件              │
│    解析 TOML，获取 vscode 配置       │
└─────────────────┬───────────────────┘
                  │
                  ▼
┌─────────────────────────────────────┐
│       3. 遍历 sources 列表          │
│    对 vscode 的每个 source 执行      │
└─────────────────┬───────────────────┘
                  │
                  ▼
┌─────────────────────────────────────┐
│       4. 路径解析                    │
│  <home>/AppData/Roaming/Code        │
│  → C:/Users/xxx/AppData/Roaming/Code│
└─────────────────┬───────────────────┘
                  │
                  ▼
┌─────────────────────────────────────┐
│       5. 文件操作                    │
│  5.1 检测源路径是否存在              │
│  5.2 检测目标路径是否存在             │
│  5.3 根据 on_exists 处理            │
│  5.4 移动文件到目标位置               │
│  5.5 在源位置创建链接                │
│  5.6 验证链接有效性                  │
└─────────────────┬───────────────────┘
                  │
                  ▼
┌─────────────────────────────────────┐
│         6. 返回结果                  │
│    打印成功/失败信息                 │
└─────────────────────────────────────┘
```

---

## 7. 错误处理

### 7.1 错误类型

| 错误类型 | 说明 |
|---------|------|
| `ConfigError` | 配置文件解析错误 |
| `PathError` | 路径解析或无效错误 |
| `FsError` | 文件系统操作错误 |
| `LinkError` | 链接创建/删除错误 |
| `WorkspaceError` | 工作区相关错误 |

### 7.2 错误处理策略

- 所有错误统一返回 `anyhow::Result`
- CLI 层负责格式化输出错误信息
- 提供 `--verbose` 选项显示详细错误栈

---

## 8. 后续演进建议

### 8.1 V1 阶段 (MVP)

- 实现核心 link/unlink 功能
- 支持基本的配置文件
- 支持软链接

### 8.2 V2 阶段

- 添加硬链接支持
- 添加 link_type 自动检测
- 添加批量操作支持

### 8.3 V3 阶段

- 添加 Web UI
- 添加配置模板市场
- 添加云端配置同步
