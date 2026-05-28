---
title: "link-disk 剩余修复任务规划"
version: "1.0.0"
date: "2026-05-28"
author: "AI Agent"
status: "plan"
project: "link-disk v1.1.0"
type: "todo-plan"
related_reports:
  - "docs/report/01-architecture-assessment.md"
  - "docs/report/02-code-optimization.md"
  - "docs/report/03-quality-report.md"
---

# link-disk 剩余修复任务规划

> 基于 `docs/report/` 三份报告的剩余建议汇总

---

## 一、剩余任务总览

| 优先级 | 任务 | 工作量 | 来源报告 |
|--------|------|--------|----------|
| 🟠 **P1** | 增加集成测试覆盖 | ~2 小时 | ✅ 完成 |
| 🟡 P2 | 占位符运行时可扩展 | ~2 小时 | ✅ 完成 |
| 🟡 P2 | 符号链接循环检测 | ~1 小时 | ✅ 完成 |
| 🟡 P2 | 配置文件权限检查 | ~0.5 小时 | ✅ 完成 |
| 🟢 P3 | 文件系统 trait 合并 (4→1) | ~1 小时 | ✅ 完成 |
| 🟢 P3 | 双层策略冲突验证 | ~0.5 小时 | ✅ 完成 |

**剩余总债务**: 0 小时 ✅ 全部完成

---

## 二、任务详情

### Phase 1: 测试覆盖 🔴 P1（约 2 小时）

**目标**: 将测试覆盖率从 15% 提升至 50%+

| 子任务 | 覆盖模块 | 工作量 | 验收标准 |
|--------|---------|--------|---------|
| 配置校验测试：占位符有效性 | `config.rs` | 15 分钟 | 无效占位符返回错误 |
| 配置校验测试：target 冲突 | `config.rs` | 15 分钟 | 相同 target 返回错误 |
| 源级别 on_exists 优先级测试 | `config.rs` + `request_builder.rs` | 20 分钟 | 源级别覆盖应用级别 |
| merge 策略合并目录测试 | `strategies.rs` + `file_mover.rs` | 30 分钟 | 目录正确合并 |
| replace 策略删除重建测试 | `strategies.rs` | 20 分钟 | 目标被删除后重建 |
| 新增集成测试 | `tests/integration_tests.rs` | 30 分钟 | 完整场景覆盖 |

**代码参考**:
- 现有集成测试: [tests/integration_tests.rs](file:///d:/Workplace/APP/Rust/link-disk/tests/integration_tests.rs)
- 现有单元测试: [path_resolver.rs:150-162](file:///d:/Workplace/APP/Rust/link-disk/src/infra/path_resolver.rs#L150-L162)

---

### Phase 2: 功能增强 🟡 P2（约 3 小时）

#### 2.1 占位符运行时可扩展（~2 小时）

**来源**: [01-architecture-assessment.md:274](file:///d:/Workplace/APP/Rust/link-disk/docs/report/01-architecture-assessment.md#L274)

**现状**: 9 个占位符定义在 `path_resolver.rs` 的 `PLACEHOLDER_REGISTRY` 中，硬编码不可扩展。

**设计方案**:
```toml
# config.toml 新增 [custom_placeholders] 段
[custom_placeholders]
workspace = "D:/link-disk-workspace"
backup = "E:/backups"
```

```rust
// path_resolver.rs 新增运行时注册
pub fn register_placeholder(key: String, value: String) {
    // 运行时写入 RwLock<HashMap>
}
```

| 子任务 | 工作量 | 涉及文件 |
|--------|--------|---------|
| 配置解析支持 custom_placeholders | 30 分钟 | config.rs |
| 运行时注册表 (RwLock) | 30 分钟 | path_resolver.rs |
| 校验逻辑扩展 | 20 分钟 | config.rs |
| 单元测试 | 30 分钟 | path_resolver.rs |

#### 2.2 符号链接循环检测（~1 小时）

**来源**: [03-quality-report.md:194](file:///d:/Workplace/APP/Rust/link-disk/docs/report/03-quality-report.md#L194)

**现状**: 创建/检测链接时未检测循环符号链接，极端深目录可能导致栈溢出。

**设计方案**:
```rust
fn detect_symlink_cycle(start: &Path, max_depth: usize) -> Result<()> {
    let mut visited = HashSet::new();
    let mut current = start;
    for _ in 0..max_depth {
        if !visited.insert(current) {
            bail!("Symlink cycle detected");
        }
        current = current.read_link()?;
    }
    Ok(())
}
```

| 子任务 | 工作量 | 涉及文件 |
|--------|--------|---------|
| 循环检测函数实现 | 30 分钟 | fs_utils.rs |
| LinkOps 中集成检测 | 20 分钟 | link_ops.rs |
| 单元测试 | 15 分钟 | fs_utils.rs |

#### 2.3 配置文件权限检查（~0.5 小时）

**来源**: [03-quality-report.md:195](file:///d:/Workplace/APP/Rust/link-disk/docs/report/03-quality-report.md#L195)

**现状**: `init` 创建配置文件后未设置文件权限，可能被其他用户读取。

**设计方案**:
```rust
#[cfg(unix)]
{
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
}
```

| 子任务 | 工作量 | 涉及文件 |
|--------|--------|---------|
| init 命令添加权限设置 | 20 分钟 | workspace.rs |
| 权限警告（现有配置） | 10 分钟 | config.rs |

---

### Phase 3: 架构优化 🟢 P3（约 1.5 小时）

#### 3.1 文件系统 trait 合并（~1 小时）

**来源**: [01-architecture-assessment.md:276](file:///d:/Workplace/APP/Rust/link-disk/docs/report/01-architecture-assessment.md#L276)

**现状**: `FsReader`、`FsCopier`、`FsWriter`、`FsLinker` 四个子 trait 从未独立使用。

**设计方案**: 合并为单个 `FileSystem` trait，保留现有方法。

```rust
// 合并前
pub trait FsReader { fn read(&self) -> ...; }
pub trait FsCopier { fn copy(&self) -> ...; }
pub trait FsWriter { fn write(&self) -> ...; }
pub trait FsLinker { fn link(&self) -> ...; }

// 合并后
pub trait FileSystem: FsReader + FsCopier + FsWriter + FsLinker {}
```

| 子任务 | 工作量 | 涉及文件 |
|--------|--------|---------|
| 定义合并后的 trait | 20 分钟 | fs_utils.rs |
| 更新所有引用 | 30 分钟 | link_ops.rs, commands/*.rs |
| 编译验证 | 10 分钟 | — |

#### 3.2 双层策略冲突验证（~0.5 小时）

**来源**: [01-architecture-assessment.md:277](file:///d:/Workplace/APP/Rust/link-disk/docs/report/01-architecture-assessment.md#L277)

**现状**: 同一应用内多个 source 使用不同策略 (`merge` + `skip`) 时，行为可能不一致。

**设计方案**:
```rust
// config.rs validate() 中新增校验
// 警告：应用内混合使用策略可能导致不一致行为
let strategies: HashSet<&str> = app_config.sources.iter()
    .map(|s| s.on_exists_strategy().unwrap_or(app_config.on_exists_strategy()))
    .collect();
if strategies.len() > 1 {
    tracing::warn!("App '{}' mixes strategies: {:?}", app_id, strategies);
}
```

| 子任务 | 工作量 | 涉及文件 |
|--------|--------|---------|
| 策略混合检测逻辑 | 20 分钟 | config.rs |
| 测试用例 | 10 分钟 | config.rs 测试 |

---

## 三、执行路线图

```
Phase 1: 测试覆盖 (P1) ───────────────── 2 小时
  └── 配置校验测试 ─── target 冲突测试 ─── 策略测试 ─── 集成测试

Phase 2: 功能增强 (P2) ───────────────── 3 小时
  ├── 占位符可扩展 ───────────────── 2 小时
  ├── 符号链接循环检测 ───────────── 1 小时
  └── 配置文件权限 ───────────────── 0.5 小时

Phase 3: 架构优化 (P3) ───────────────── 1.5 小时
  ├── 文件系统 trait 合并 ────────── 1 小时
  └── 策略冲突验证 ───────────────── 0.5 小时
────────────────────────────────────────
总计: 约 7 小时
```

---

## 四、与报告的追溯关系

| 任务 | 报告引用 |
|------|----------|
| 集成测试覆盖 | [01: §六](file:///d:/Workplace/APP/Rust/link-disk/docs/report/01-architecture-assessment.md#L270), [02: §五](file:///d:/Workplace/APP/Rust/link-disk/docs/report/02-code-optimization.md#L144), [03: §四](file:///d:/Workplace/APP/Rust/link-disk/docs/report/03-quality-report.md#L127) |
| 占位符可扩展 | [01: §六](file:///d:/Workplace/APP/Rust/link-disk/docs/report/01-architecture-assessment.md#L274), [02: §五](file:///d:/Workplace/APP/Rust/link-disk/docs/report/02-code-optimization.md#L163), [03: §六](file:///d:/Workplace/APP/Rust/link-disk/docs/report/03-quality-report.md#L196) |
| 文件系统 trait 合并 | [01: §三](file:///d:/Workplace/APP/Rust/link-disk/docs/report/01-architecture-assessment.md#L183), [02: §五](file:///d:/Workplace/APP/Rust/link-disk/docs/report/02-code-optimization.md#L164), [03: §六](file:///d:/Workplace/APP/Rust/link-disk/docs/report/03-quality-report.md#L197) |
| 循环链接检测 | [03: §六](file:///d:/Workplace/APP/Rust/link-disk/docs/report/03-quality-report.md#L194) |
| 配置文件权限 | [03: §六](file:///d:/Workplace/APP/Rust/link-disk/docs/report/03-quality-report.md#L195) |
| 策略冲突验证 | [01: §六](file:///d:/Workplace/APP/Rust/link-disk/docs/report/01-architecture-assessment.md#L277) |

---

## 五、完成进度

- [x] **Phase 1: 测试覆盖** (6/6 ✅)
  - [x] 配置校验：占位符有效性测试
  - [x] 配置校验：target 冲突测试
  - [x] 源级别 on_exists 优先级测试
  - [x] merge 策略合并目录测试
  - [x] replace 策略删除重建测试
  - [x] 完善集成测试

- [x] **Phase 2: 功能增强** (5/5 ✅)
  - [x] 配置解析支持 custom_placeholders
  - [x] 运行时注册表 (RwLock)
  - [x] 符号链接循环检测函数
  - [x] 链接操作中集成循环检测
  - [x] 配置文件权限设置

- [x] **Phase 3: 架构优化** (2/2 ✅)
  - [x] 文件系统 trait 合并
  - [x] 双层策略冲突验证

---

**规划生成时间**: 2026-05-28
**规划工具**: AI Agent
**规划版本**: v1.0.0