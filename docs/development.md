# link-disk 开发路线图

## 概述

本文档描述 link-disk 项目的开发阶段划分和当前状态。

---

## 开发阶段总览

| 阶段 | 版本 | 目标 | 状态 |
|------|------|------|------|
| Phase 1 | V0.1 | MVP - 核心功能 | ✅ 完成 |
| Phase 2 | V0.2 | 增强功能 | ✅ 完成 |
| Phase 3 | V1.0 | 稳定发布 | ✅ 完成 |

---

## Phase 1: MVP 实现 (V0.1) ✅ 完成

### 目标

实现最基本的功能：能够通过配置文件转移文件夹并创建软链接。

### 功能列表

- [x] CLI 基础框架搭建 (clap)
- [x] 配置文件解析 (TOML)
- [x] 路径占位符解析 (`<home>` 等)
- [x] 软链接/硬链接创建操作
- [x] 基本的 link 命令

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

## Phase 2: 增强功能 (V0.2) ✅ 完成

### 目标

完善功能，增加 unlink、status、repair 等命令，提升用户体验。

### 功能列表

- [x] unlink 命令 (移除链接，恢复原状)
- [x] status 命令 (检查链接状态)
- [x] repair 命令 (修复损坏的链接)
- [x] --dry-run 选项 (模拟运行)
- [x] --verbose 选项 (详细输出)
- [x] --force 选项 (强制处理)
- [x] 硬链接支持
- [x] on_exists 策略完善 (skip/merge/replace/overwrite)

### on_exists 策略说明

| 策略 | 行为 | 适用场景 |
|------|------|---------|
| **Skip** | 跳过，存在则不操作（默认） | 保留 target 的现有数据，避免覆盖 |
| **Merge** | 合并源到目标后删除源，继续创建链接 | 以 target 为准，以 source 补充 |
| **Overwrite** | 删除源后继续创建链接 | 确认 source 数据不再需要，保留目标数据 |
| **Replace** | 删除目标，移动源到目标，创建链接 | 确认 target 数据不再需要，可以完全替换 |

### Phase 2 验收标准

- [x] `link-disk unlink` 能够正确移除链接并恢复文件
- [x] `link-disk status` 能够准确报告链接状态
- [x] `link-disk repair` 能够修复损坏的链接
- [x] `--dry-run` 选项能够模拟运行不实际执行
- [x] `--verbose` 选项能够显示详细操作过程
- [x] `--force` 选项能够强制处理已存在的符号链接
- [x] 支持硬链接创建
- [x] 所有 on_exists 策略正常工作

---

## Phase 3: 稳定发布 (V1.0) ✅ 完成

### 目标

提升稳定性和用户体验，准备正式发布。

### 功能状态

| 功能 | 状态 | 说明 |
|------|------|------|
| 完善的错误处理 | ✅ 完成 | 使用 anyhow + 自定义错误类型 |
| 详细调试日志 | ✅ 完成 | --verbose 显示详细步骤信息 |
| 默认配置模板 | ✅ 完成 | init 命令生成默认配置 |
| Windows 特殊处理 | ✅ 完成 | 符号链接安全删除 |
| 单元测试 | 🔄 进行中 | path_resolver 有测试 |
| 代码注释 | ✅ 完成 | 所有模块添加完整文档注释 |
| 业务流程文档 | ✅ 完成 | workflows.md 完整流程图 |

### 已完成子任务

#### 3.1 错误处理 ✅

```rust
// error.rs - 自定义错误类型
enum LinkDiskError {
    Io(std::io::Error),
    Config(String),
    Path(String),
    Link(String),
}
// 已实现: Display, Debug, From<io::Error>
// 包含: validate_path() 工具函数
```

当前使用 anyhow::Result 统一错误处理：
- 通过 `.with_context()` 为底层 IO 错误添加可读的操作描述
- 通过 `.bail!()` 创建上下文丰富的错误
- 在 `main()` 中统一捕获并输出错误信息

#### 3.2 跨平台支持 ✅

- CLI 参数解析跨平台 (clap)
- 路径处理使用 `std::path`
- 符号链接操作已区分 Windows/Unix
- Windows 符号链接特殊删除逻辑

#### 3.3 调试功能 ✅

- `--verbose` 选项显示详细操作步骤
- 显示源路径、目标路径、链接类型、force 状态
- 显示每个操作的执行结果

#### 3.4 配置文件模板 ✅

`init` 命令自动生成包含示例的默认配置文件：
- VSCode 示例配置
- Chrome 示例配置
- 注释说明各字段用途

#### 3.5 Windows 特殊处理 ✅

- **符号链接安全删除**: 先尝试 remove_dir（目录符号链接），失败则 remove_file（文件符号链接）
- **权限问题提示**: 错误消息清晰说明原因
- **路径格式**: 统一使用反斜杠

#### 3.6 文档完善 ✅

- architecture.md - 架构设计文档
- config.md - 配置文件详细说明
- development.md - 开发路线图（本文件）
- manual.md - 用户使用手册
- workflows.md - 业务流程文档
- README.md - 项目介绍和使用指南

---

## 代码质量状态

| 指标 | 状态 | 说明 |
|------|------|------|
| 编译警告 | ✅ 零警告 | cargo build 无警告 |
| Clippy | ✅ 零警告 | cargo clippy 无警告 |
| 架构设计 | ✅ 良好 | 分层清晰，SOLID 合规 |
| 代码重复 | ✅ 无 | fs_utils 统一封装 |
| 代码注释 | ✅ 完整 | 所有模块有模块级、函数级注释 |
| 依赖管理 | ✅ 正常 | clap/toml/serde/anyhow/dirs/spinners |

---

## 技术债务与未来计划

### 未来考虑

- [ ] 日志功能增强（输出到文件）
- [ ] 配置文件加密存储
- [ ] 云端配置同步
- [ ] Web UI 管理界面
- [ ] 批量迁移工具
- [ ] 配置导入/导出
- [ ] 更完整的单元测试覆盖
- [ ] 集成测试自动化

---

## 快速参考

### 当前可用命令

```bash
# 全局选项
link-disk [-v, --verbose] [-c, --config <PATH>] <命令> [选项]

# 子命令
link-disk init [--path <路径>] [--force]
link-disk link [应用名...] [--all] [--dry-run] [--force] [-v]
link-disk unlink [应用名...] [--all] [--force] [-k, --keep-files]
link-disk list [--app <应用名>]
link-disk status [应用名...]
link-disk repair [应用名...] [--force]
```

### 当前版本信息

- 当前版本: v1.0+
- Rust Edition: 2024
- 支持平台: Windows (主要), Linux/macOS (待充分测试)

### 项目结构

```
src/
├── main.rs              # 程序入口
├── cli.rs               # CLI 命令定义
├── config.rs            # 配置解析
├── workspace.rs         # 工作区管理
├── link_ops.rs          # 链接操作
├── path_resolver.rs     # 路径解析
├── fs_utils.rs          # 文件系统工具
└── error.rs             # 错误类型
```
