# link-disk 开发路线图

## 概述

本文档描述 link-disk 项目的开发阶段划分和具体实现计划。

---

## 开发阶段总览

| 阶段 | 版本 | 目标 | 优先级 |
|------|------|------|--------|
| Phase 1 | V0.1 | MVP - 核心功能 | P0 |
| Phase 2 | V0.2 | 增强功能 | P1 |
| Phase 3 | V1.0 | 稳定发布 | P1 |

---

## Phase 1: MVP 实现 (V0.1)

### 目标

实现最基本的功能：能够通过配置文件转移文件夹并创建软链接。

### 功能列表

- [x] CLI 基础框架搭建
- [x] 配置文件解析 (TOML)
- [x] 路径占位符解析 (`<home>` 等)
- [x] 软链接创建操作
- [x] 基本的 link 命令

### 详细任务

#### 1.1 项目初始化

```
任务:
- [ ] 确认 Cargo.toml 配置正确
- [ ] 添加必要依赖 (clap, toml, anyhow)
- [ ] 创建基础目录结构
```

**依赖库选择:**

| 库 | 版本 | 用途 |
|----|------|------|
| clap | 4.x | CLI 参数解析 |
| toml | 0.8 | TOML 配置解析 |
| anyhow | 1.x | 错误处理 |
| serde | 1.x | 序列化/反序列化 |

#### 1.2 CLI 层实现

```
模块: src/cli.rs
任务:
- [ ] 定义 CLI 参数结构
- [ ] 实现 init 子命令
- [ ] 实现 link 子命令
- [ ] 实现 list 子命令
```

**命令设计:**

```bash
link-disk init [--path <路径>]
link-disk link [--all] [应用名...]
link-disk list
```

#### 1.3 配置层实现

```
模块: src/config.rs
任务:
- [ ] 定义配置数据结构
- [ ] 实现配置文件加载
- [ ] 实现配置验证
```

**数据结构:**

```rust
struct Config {
    workspace: Workspace,
    apps: HashMap<String, AppConfig>,
}

struct Workspace {
    path: PathBuf,
}

struct AppConfig {
    name: String,
    enabled: bool,
    on_exists: OnExists,
    sources: Vec<Source>,
}

struct Source {
    source: String,      // 支持占位符
    target: String,
    link_type: LinkType,
}
```

#### 1.4 路径解析实现

```
模块: src/path_resolver.rs
任务:
- [ ] 定义支持的占位符列表
- [ ] 实现占位符到实际路径的转换
- [ ] 处理路径规范化
```

**占位符列表:**

```rust
const PLACEHOLDERS: &[(&str, &str)] = &[
    ("<home>", get_user_home_dir()),
    ("<appdata>", get_appdata_dir()),
    ("<localappdata>", get_local_appdata_dir()),
    ("<documents>", get_documents_dir()),
    ("<desktop>", get_desktop_dir()),
    ("<downloads>", get_downloads_dir()),
    ("<temp>", get_temp_dir()),
    ("<programfiles>", get_program_files_dir()),
    ("<programfilesx86>", get_program_files_x86_dir()),
];
```

#### 1.5 文件系统操作实现

```
模块: src/fs_utils.rs
任务:
- [ ] 实现目录递归创建
- [ ] 实现目录移动 (move_dir)
- [ ] 实现软链接创建
- [ ] 实现链接类型检测
```

#### 1.6 Link 操作实现

```
模块: src/link_ops.rs
任务:
- [ ] 实现 link_single_source()
- [ ] 实现 link_app()
- [ ] 实现 link_all()
- [ ] 实现进度输出
```

### Phase 1 验收标准

- [x] `link-disk init` 能够创建工作区和配置文件
- [x] 配置文件能够正确解析 TOML 格式
- [x] `<home>` 等占位符能够正确替换
- [x] `link-disk link vscode` 能够:
  - 解析源路径和目标路径
  - 将源文件夹移动到目标位置
  - 在源位置创建软链接
- [x] `link-disk list` 能够列出已配置的链接

---

## Phase 2: 增强功能 (V0.2)

### 目标

完善功能，增加 unlink、status、repair 等命令，提升用户体验。

### 功能列表

- [x] unlink 命令 (移除链接，恢复原状)
- [x] status 命令 (检查链接状态)
- [x] repair 命令 (修复损坏的链接)
- [x] --dry-run 选项 (模拟运行)
- [x] --verbose 选项 (详细输出)
- [x] 硬链接支持
- [x] on_exists 策略完善 (skip/merge/replace)

### 详细任务

#### 2.1 Unlink 命令

```bash
link-disk unlink [--all] [应用名...]
link-disk unlink [--force] [应用名...]
```

**实现逻辑:**

```
1. 解析配置，获取应用列表
2. 对每个应用:
   a. 删除软链接/硬链接
   b. 将目标文件夹移回源位置
3. 输出操作结果
```

#### 2.2 Status 命令

```bash
link-disk status [应用名...]
```

**链接状态定义:**

| 状态 | 条件 |
|------|------|
| 正常 | 链接存在且指向有效目标 |
| 损坏 | 链接存在但目标不存在 |
| 孤立 | 目标存在但链接不存在 |
| 未链接 | 尚未执行过 link 操作 |

#### 2.3 Repair 命令

```bash
link-disk repair [--force] [应用名...]
```

**修复策略:**

```
1. 检测链接状态
2. 对于损坏的链接:
   a. 删除损坏链接
   b. 重新创建链接
3. 对于孤立文件:
   a. 提示用户是否创建链接
```

#### 2.4 硬链接支持

```toml
[[apps.example.sources]]
source = "<home>/path/to/folder"
target = "example/folder"
link_type = "hardlink"  # 或 symlink
```

**注意:** 硬链接仅支持同文件系统，需要在操作前检测。

#### 2.5 on_exists 策略完善

```rust
enum OnExists {
    Skip,      # 跳过，存在则不操作
    Merge,     # 合并内容
    Replace,   # 替换目标
}
```

### Phase 2 验收标准

- [x] `link-disk unlink` 能够正确移除链接并恢复文件
- [x] `link-disk status` 能够准确报告链接状态
- [x] `link-disk repair` 能够修复损坏的链接
- [x] `--dry-run` 选项能够模拟运行不实际执行
- [x] `--verbose` 选项能够显示详细操作过程
- [x] 支持硬链接创建

---

## Phase 3: 稳定发布 (V1.0)

### 目标

提升稳定性和用户体验，准备正式发布。

### 功能列表

- [ ] 完善的错误处理和提示
- [ ] 日志记录功能
- [ ] 配置文件模板
- [ ] Windows 特殊处理 (权限、开发者模式提示)
- [ ] 单元测试和集成测试
- [ ] 跨平台支持 (Linux/macOS)

### 详细任务

#### 3.1 错误处理

```rust
enum LinkDiskError {
    ConfigError(String),
    PathError(String),
    FsError(String),
    LinkError(String),
    PermissionError(String),
}
```

**改进措施:**
- 统一的错误类型
- 友好的错误提示
- 提供解决建议

#### 3.2 日志功能

```bash
link-disk --log-level debug link vscode
```

**日志内容:**
- 操作时间
- 操作类型
- 源路径 → 目标路径
- 操作结果
- 错误详情 (如果失败)

#### 3.3 配置文件模板

提供常用应用的预设配置:

```bash
# 使用模板创建配置
link-disk init --template vscode

# 可用模板
# - vscode
# - chrome
# - jetbrains
# - nodejs
```

#### 3.4 Windows 特殊处理

```
任务:
- [ ] 检测是否以管理员权限运行
- [ ] 检测开发者模式是否开启
- [ ] 提供友好的权限问题提示
- [ ] 正确处理 Windows 路径
```

#### 3.5 测试

```
任务:
- [ ] 单元测试 (path_resolver, config 解析)
- [ ] 集成测试 (完整 link/unlink 流程)
- [ ] 跨平台测试 (Windows/Linux/macOS)
```

#### 3.6 跨平台支持

| 平台 | 占位符差异 |
|------|-----------|
| Windows | `<home>`, `<appdata>`, `<programfiles>` |
| Linux | `<home>`, `<config>`, `<data>` |
| macOS | `<home>`, `<library>`, `<application support>` |

### Phase 3 验收标准

- [ ] 所有命令稳定运行，无 panic
- [ ] 错误提示友好，提供解决建议
- [ ] 日志记录完整
- [ ] 配置文件模板可用
- [ ] Windows 上权限问题有清晰提示
- [ ] 测试覆盖率达到 80%+
- [ ] 能够在 Linux/macOS 上正常运行

---

## 技术债务

### 未来考虑

- [ ] 配置文件加密存储
- [ ] 云端配置同步
- [ ] Web UI 管理界面
- [ ] 批量迁移工具
- [ ] 配置导入/导出

---

## 开发优先级

```
P0 (必须):
1. CLI 框架
2. 配置解析
3. 路径解析
4. 软链接操作

P1 (重要):
5. Unlink 命令
6. Status 命令
7. Dry-run 支持
8. 错误处理

P2 (优化):
9. Repair 命令
10. 日志功能
11. 模板支持
12. 测试
```

---

## 里程碑

| 日期 | 里程碑 | 说明 |
|------|--------|------|
| Week 1 | Phase 1 完成 | 核心 link 功能可用 |
| Week 2 | Phase 2 完成 | 命令完善，基本可用 |
| Week 3 | Phase 3 完成 | 稳定发布 |
