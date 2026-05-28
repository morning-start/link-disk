# 已知问题清单

> **版本**: v1.1.0
> **更新日期**: 2026-05-29
> **来源**: repo-analyzer + code-optimizer 三轮深度分析（01-architecture-assessment / 02-code-optimization / 03-quality-report）
> **说明**: 以下问题均在分析过程中发现，因影响范围大、依赖外部变更或属于设计取舍，在本版本中暂不修复。

---

## 一、架构问题

### ARCH-01: 层间泄漏 — Commands 层直接引用 FsUtils

| 属性 | 内容 |
|------|------|
| **严重度** | 🟡 中 |
| **来源** | [01-architecture-assessment.md](report/01-architecture-assessment.md) §2.3 |
| **文件** | [commands/link.rs](src/commands/link.rs) |
| **描述** | Commands 层直接引用 `FsUtils`（属于 Infra 层），违反了"上层依赖下层"的三层架构原则。正确的做法应该是 Commands → Domain(LinkOps) → Infra(FsUtils) 的链式调用。 |
| **影响** | 添加新命令时需要了解底层文件系统接口，增加认知负担；违反分层隔离，未来替换文件系统实现时波及范围更大。 |
| **修复思路** | 将 FsUtils 调用下沉到 Domain 层，Commands 层只通过 LinkOps/Strategies 间接操作文件系统。 |

### ARCH-02: 缺少 MockFileSystem 测试基础设施

| 属性 | 内容 |
|------|------|
| **严重度** | 🟡 中 |
| **来源** | [01-architecture-assessment.md](report/01-architecture-assessment.md) §8.2 |
| **文件** | 全局 |
| **描述** | 所有测试依赖真实文件系统，无法做纯内存的单元级别模拟测试。集成测试虽然覆盖了核心路径，但边界条件（磁盘满、权限拒绝等）难以触发。 |
| **影响** | 错误路径测试覆盖率不足；测试运行依赖磁盘 I/O，速度慢。 |
| **修复思路** | 实现 `MockFileSystem` 实现 `FileSystem` trait，在测试中替代真实文件系统。 |

### ARCH-03: status 命令输出不可编程消费

| 属性 | 内容 |
|------|------|
| **严重度** | 🟢 低 |
| **来源** | [01-architecture-assessment.md](report/01-architecture-assessment.md) §8.4 |
| **文件** | [commands/status.rs](src/commands/status.rs) |
| **描述** | `status` 命令直接输出人类可读文本（如 `[OK] vscode`），没有提供 JSON 等结构化输出格式。其他工具难以解析和消费输出。 |
| **影响** | 无法被 GUI 前端或其他脚本工具集成。 |
| **修复思路** | 添加 `--json` 或 `--format` 参数，支持 JSON/YAML 结构化输出。 |

---

## 二、代码质量问题

### CODE-01: TOCTOU 竞争条件（符号链接创建）

| 属性 | 内容 |
|------|------|
| **严重度** | 🟢 低 |
| **来源** | [02-code-optimization.md](report/02-code-optimization.md) §4.4 |
| **文件** | [domain/link_ops.rs](src/domain/link_ops.rs) |
| **描述** | 在 `create_link` 函数中，`detect_symlink_cycle` 在创建链接前检测循环，检查后到创建之间有一个微小时间窗口。理论上外部进程可在此窗口创建恶意链接。 |
| **影响** | CLI 工具的典型使用场景下概率极低，标记为"可接受风险"。仅当多进程同时操作同一路径时才存在理论可能。 |
| **修复思路** | 使用 Windows 的 `NtCreateSymbolicLink` / Linux 的 `O_NOFOLLOW` 原子操作代替"检查后创建"模式。 |

### CODE-02: from_str_lossy 模式轻微重复

| 属性 | 内容 |
|------|------|
| **严重度** | 🟢 低 |
| **来源** | [02-code-optimization.md](report/02-code-optimization.md) §3.2 |
| **文件** | [domain/link_ops.rs](src/domain/link_ops.rs#L61) / [domain/strategies.rs](src/domain/strategies.rs#L185) |
| **描述** | `LinkType` 和 `OnExists` 都有相同的 `from_str_lossy` 实现模式：`FromStr::from_str(s).unwrap_or(default)`。两个类型的行为不同，但模式重复。 |
| **影响** | 添加新的可解析类型时需要重复编写相同模式，轻微违反 DRY。 |
| **修复思路** | 提取 `FromStrLossy` trait，提供默认实现。 |

### CODE-03: 编译警告 — 公共 API 导出未使用

| 属性 | 内容 |
|------|------|
| **严重度** | 🟢 低 |
| **来源** | [02-code-optimization.md](report/02-code-optimization.md) §2.3 |
| **文件** | [infra/mod.rs](src/infra/mod.rs#L13) |
| **描述** | `register_placeholder` 和 `is_known_placeholder` 通过 `pub use` 导出供集成测试和外部使用者调用，但 crate 内部未直接使用这些路径，产生编译警告。 |
| **影响** | 运行时无影响，仅编译期产生一个警告。 |
| **修复思路** | 将这两个函数移入 `lib.rs` 的公共导出，避免 infra/mod.rs 层面的 `#[allow(unused_imports)]`。 |

---

## 三、测试覆盖缺口

### TEST-01: 命令层缺少单元测试

| 属性 | 内容 |
|------|------|
| **严重度** | 🟠 较高 |
| **来源** | [03-quality-report.md](report/03-quality-report.md) §3.3 |
| **文件** | [commands/](src/commands/) 目录 |
| **描述** | 6 个命令（init/link/unlink/list/status/repair）只有集成测试覆盖完整流程，没有独立的单元测试来验证各函数的边界条件和错误处理路径。 |
| **影响** | 命令层代码改动后缺少快速反馈的测试保障。 |
| **修复思路** | 为每个命令添加 `#[cfg(test)]` 模块，测试配置解析失败、工作区不存在等异常路径。 |

### TEST-02: 错误路径边缘覆盖不足

| 属性 | 内容 |
|------|------|
| **严重度** | 🟠 较高 |
| **来源** | [03-quality-report.md](report/03-quality-report.md) §3.3 |
| **相关模块** | link_ops.rs, file_mover.rs, fs_utils.rs |
| **描述** | 当前测试主要覆盖"快乐路径"（Happy Path），对以下场景覆盖率不足：<br>- 目标路径已存在且不可删除<br>- 磁盘空间不足<br>- 权限拒绝<br>- 源路径不存在 |
| **影响** | 生产环境中的异常场景可能未被充分验证。 |
| **修复思路** | 实现 MockFileSystem（见 ARCH-02），在模拟环境中触发上述异常条件。 |

### TEST-03: 测试覆盖率 65% 未达质量门禁

| 属性 | 内容 |
|------|------|
| **严重度** | 🟡 中 |
| **来源** | [03-quality-report.md](report/03-quality-report.md) §7 |
| **文件** | 全局 |
| **描述** | 当前测试覆盖率约 **65%**，低于质量门禁阈值 **80%**。这是 4 项质量门禁中唯一未达标项。 |
| **影响** | 重构时缺乏充分的回归安全保障。 |
| **修复思路** | 优先补齐命令层单元测试（TEST-01），可将覆盖率提升至 75%+。 |

---

## 四、文档陈旧

### DOC-01: docs/architecture.md 严重过时

| 属性 | 内容 |
|------|------|
| **严重度** | 🟡 中 |
| **来源** | [01-architecture-assessment.md](report/01-architecture-assessment.md) §1.3 |
| **文件** | [docs/architecture.md](docs/architecture.md) |
| **描述** | 架构文档仍反映旧版单文件架构（main.rs 包含所有逻辑），与当前的三层架构（commands/domain/infra）完全不符。 |
| **影响** | 新加入开发者如果仅看 architecture.md 会被严重误导。 |
| **修复思路** | 按照 AGENTS.md 中的架构分层重写 architecture.md。 |

### DOC-02: docs/config.md 需同步更新

| 属性 | 内容 |
|------|------|
| **严重度** | 🟢 低 |
| **来源** | [01-architecture-assessment.md](report/01-architecture-assessment.md) §8.5 |
| **文件** | [docs/config.md](docs/config.md) |
| **描述** | 配置文档缺少 `custom_placeholders` 和策略冲突检测说明。 |
| **影响** | 用户无法从文档了解自定义占位符和策略冲突检测功能。 |
| **修复思路** | 补充 custom_placeholders 配置说明和策略优先级的完整描述。 |

### DOC-03: docs/workflows.md 陈旧

| 属性 | 内容 |
|------|------|
| **严重度** | 🟢 低 |
| **来源** | 分析过程中发现 |
| **文件** | [docs/workflows.md](docs/workflows.md) |
| **描述** | 业务流程文档需更新以反映修复命令、策略冲突检测等新功能。 |
| **影响** | 新功能缺少流程图说明。 |
| **修复思路** | 补充 repair 命令流程和策略冲突检测流程图。 |

---

## 五、安全增强建议

### SEC-01: on_exists 策略链式检测

| 属性 | 内容 |
|------|------|
| **严重度** | 🟢 低 |
| **来源** | [01-architecture-assessment.md](report/01-architecture-assessment.md) §8.5 |
| **文件** | [infra/config.rs](src/infra/config.rs) |
| **描述** | 当前策略冲突检测仅检查"应用级 vs 源级"之间的直接矛盾，未做更细粒度的全局链式检测（例如：同一应用中 A-source=merge 和 B-source=skip 且二者共享子目录时，A 的操作可能破坏 B 的预期行为）。 |
| **影响** | 极端复杂配置下可能出现意外的策略交互。 |
| **修复思路** | 在 `check_strategy_conflicts` 中添加跨 source 的 path-overlap 冲突检测。 |

---

## 汇总

| 编号 | 问题 | 严重度 | 优先级 | 预估工作量 |
|------|------|--------|--------|-----------|
| ARCH-01 | 层间泄漏 FsUtils | 🟡 中 | P3 | ~2 小时 |
| ARCH-02 | 缺少 MockFileSystem | 🟡 中 | P2 | ~3 小时 |
| ARCH-03 | status 输出不可编程 | 🟢 低 | P4 | ~1 小时 |
| CODE-01 | TOCTOU 竞争条件 | 🟢 低 | P4 | ~1 小时 |
| CODE-02 | from_str_lossy 重复 | 🟢 低 | P4 | ~0.5 小时 |
| CODE-03 | 编译警告 | 🟢 低 | P4 | ~15 分钟 |
| TEST-01 | 命令层无单元测试 | 🟠 较高 | **P1** | ~2 小时 |
| TEST-02 | 错误路径覆盖不足 | 🟠 较高 | **P1** | ~2 小时 |
| TEST-03 | 覆盖率未达门禁 80% | 🟡 中 | P2 | ~4 小时 |
| DOC-01 | architecture.md 过时 | 🟡 中 | P2 | ~1 小时 |
| DOC-02 | config.md 需更新 | 🟢 低 | P3 | ~0.5 小时 |
| DOC-03 | workflows.md 陈旧 | 🟢 低 | P3 | ~0.5 小时 |
| SEC-01 | 策略链式检测 | 🟢 低 | P4 | ~1 小时 |

**总计**: ~18.5 小时