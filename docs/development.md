# link-disk 开发路线图

## 概述

本文档描述 link-disk 项目的开发阶段划分和当前状态。

---

## 开发阶段总览

| 阶段 | 版本 | 目标 | 状态 |
|------|------|------|------|
| Phase 1 | V0.1 | MVP - 核心功能 | ✅ 完成 |
| Phase 2 | V0.2 | 增强功能 | ✅ 完成 |
| Phase 3 | V1.0 | 稳定发布 | 🔄 进行中 |

---

## Phase 1: MVP 实现 (V0.1) ✅ 完成

### 目标

实现最基本的功能：能够通过配置文件转移文件夹并创建软链接。

### 功能列表

- [x] CLI 基础框架搭建
- [x] 配置文件解析 (TOML)
- [x] 路径占位符解析 (`<home>` 等)
- [x] 软链接创建操作
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
- [x] 硬链接支持
- [x] on_exists 策略完善 (skip/merge/replace)

### 链接状态定义

| 状态 | 条件 |
|------|------|
| `linked` | 链接存在且指向有效目标 |
| `broken` | 链接存在但目标不存在 |
| `target_only` | 目标存在但链接不存在 |
| `both_exist` | 源和目标都存在 (非链接) |
| `source_only` | 只有源存在 |
| `none` | 都不存在 |

### on_exists 策略

```rust
enum OnExists {
    Skip,      // 跳过，存在则不操作
    Merge,     // 合并内容后删除源目录，跳过 move，直接创建链接
    Overwrite, // 删除源，跳过 move，直接创建链接
    Replace,   // 删除目标，移动源到目标位置，创建链接
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

## Phase 3: 稳定发布 (V1.0) 🔄 进行中

### 目标

提升稳定性和用户体验，准备正式发布。

### 功能状态

| 功能 | 状态 | 说明 |
|------|------|------|
| 完善的错误处理 | 🔄 | 使用 anyhow，当前可用 |
| 日志记录功能 | ❌ | 待实现 |
| 配置文件模板 | ❌ | 待实现 |
| Windows 特殊处理 | 🔄 | 基础支持 |
| 单元测试和集成测试 | 🔄 | path_resolver 有简单测试 |
| 跨平台支持 (Linux/macOS) | 🔄 | 代码兼容，需测试 |

### 已完成子任务

#### 3.1 错误处理 ✅

```rust
// error.rs 已定义 (预留，当前使用 anyhow)
enum LinkDiskError {
    Io(std::io::Error),
    Config(String),
    Path(String),
    Link(String),
}
// 已实现: Display, Debug, From<io::Error>
// 包含: validate_path() 工具函数
```

#### 3.2 跨平台支持 ✅

- CLI 参数解析跨平台
- 路径处理使用 `std::path`
- 符号链接操作已区分 Windows/Unix

### 待完成子任务

#### 3.3 日志功能

```
待实现:
- [ ] 日志级别配置 (debug/info/warn/error)
- [ ] 日志输出到文件
- [ ] 日志格式规范
```

#### 3.4 配置文件模板

```bash
# 未来计划
link-disk init --template vscode
```

#### 3.5 Windows 特殊处理

```
待实现:
- [ ] 检测是否以管理员权限运行
- [ ] 检测开发者模式是否开启
- [ ] 提供友好的权限问题提示
```

#### 3.6 测试覆盖

```
当前状态:
- path_resolver.rs: 有简单测试
- 其他模块: 待补充

待实现:
- [ ] fs_utils 单元测试
- [ ] link_ops 单元测试
- [ ] 集成测试
```

---

## 代码质量状态

| 指标 | 状态 | 说明 |
|------|------|------|
| 编译警告 | ✅ 零警告 | cargo build 无警告 |
| Clippy | ✅ 零警告 | cargo clippy 无警告 |
| 架构设计 | ✅ 良好 | 分层清晰，SOLID 合规 |
| 代码重复 | ✅ 无 | fs_utils 统一封装 |
| 依赖管理 | ✅ 正常 | clap/toml/serde/anyhow/dirs/spinners |

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
P0 (必须): ✅ 已完成
1. CLI 框架
2. 配置解析
3. 路径解析
4. 软链接操作

P1 (重要): ✅ 已完成
5. Unlink 命令
6. Status 命令
7. Dry-run 支持
8. 错误处理

P2 (优化): 🔄 进行中
9. Repair 命令 ✅
10. 日志功能 ❌
11. 模板支持 ❌
12. 测试 🔄
```

---

## 里程碑

| 日期 | 里程碑 | 状态 |
|------|--------|------|
| Week 1 | Phase 1 完成 | ✅ 完成 |
| Week 2 | Phase 2 完成 | ✅ 完成 |
| Week 3 | Phase 3 测试完善 | 🔄 进行中 |
| Week 4 | Phase 3 稳定发布 | 待开始 |

---

## 快速参考

### 当前可用命令

```bash
link-disk init [--path <路径>] [--force]
link-disk link [--all] [应用名...] [--dry-run] [--verbose]
link-disk unlink [--all] [应用名...] [--force] [--keep-files]
link-disk list [--app <应用名>]
link-disk status [应用名...]
link-disk repair [应用名...] [--force]
```

### 当前支持占位符

| 占位符 | 说明 |
|--------|------|
| `<home>` | 用户主目录 |
| `<appdata>` | AppData/Roaming |
| `<localappdata>` | AppData/Local |
| `<documents>` | 文档文件夹 |
| `<desktop>` | 桌面 |
| `<downloads>` | 下载文件夹 |
| `<temp>` | 临时文件夹 |
| `<programfiles>` | Program Files |
| `<programfilesx86>` | Program Files (x86) |
