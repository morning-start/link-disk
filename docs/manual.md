# link-disk 使用手册

## 1. 安装

### 1.1 从源码编译

```bash
git clone https://github.com/yourname/link-disk.git
cd link-disk
cargo build --release
# 可执行文件会在 target/release/link-disk.exe (Windows) 或 target/release/link-disk (Unix)
```

### 1.2 环境要求

- Rust 1.75+ (2024 Edition)
- Windows: 需要管理员权限或开发者模式创建符号链接
- Linux/macOS: 无特殊权限要求

### 1.3 添加到 PATH

将可执行文件所在目录添加到系统 PATH 环境变量。

---

## 2. 快速开始

### 2.1 初始化工作区

```bash
link-disk init
```

这会创建：

- 配置文件：`~/.link-disk/config.toml`
- 工作区文件夹：`D:/link-disk-workspace/`（或其他配置的路径）

**选项：**

| 选项 | 说明 |
|------|------|
| `--path <路径>` | 指定工作区路径 |
| `--force` | 如果已存在，强制重新初始化 |

### 2.2 编辑配置文件

根据 [配置文件说明](config.md) 编辑 `~/.link-disk/config.toml`，添加需要管理的应用。

### 2.3 创建链接

```bash
# 为所有配置的应用创建链接
link-disk link --all

# 为指定应用创建链接
link-disk link vscode
link-disk chrome

# 多个应用
link-disk link vscode chrome
```

### 2.4 查看状态

```bash
# 查看所有链接状态
link-disk status

# 查看详细信息（带调试输出）
link-disk status -v
```

---

## 3. 命令详解

### 3.1 init - 初始化工作区

初始化工作区目录和默认配置文件。

```bash
link-disk init [选项]
```

**选项：**

| 选项 | 说明 |
|------|------|
| `--path <路径>` | 指定工作区路径 |
| `--force` | 如果已存在，强制重新初始化 |

**示例：**

```bash
# 使用默认路径初始化
link-disk init

# 指定工作区路径
link-disk init --path "E:/my-workspace"

# 强制重新初始化
link-disk init --force
```

---

### 3.2 link - 创建链接

将应用的配置和数据文件夹转移到工作区，并在原位置创建符号链接。

```bash
link-disk link [应用名...] [选项]
```

**参数：**

- 不指定应用名 + 不使用 `--all`：提示无应用可处理
- 使用 `--all`：处理所有已启用的应用
- 指定应用名：只处理指定的应用

**选项：**

| 选项 | 说明 |
|------|------|
| `--all`, `-a` | 处理所有已配置的应用 |
| `--dry-run`, `-d` | 模拟运行，不实际执行操作 |
| `--force`, `-f` | 强制处理（删除已存在的软链接后重新链接） |
| `--verbose`, `-v` | 显示详细过程信息 |

**link 逻辑说明：**

| 情况                         | 处理方式                                                      |
| ---------------------------- | ------------------------------------------------------------- |
| source 存在，target 不存在   | 移动 source → target，创建链接                                |
| source 存在，target 存在     | 根据 on_exists 策略处理冲突，然后移动（如需要）并创建链接     |
| source 不存在，target 不存在 | 创建 target 目录，创建链接                                    |
| source 是损坏的符号链接      | 如果 force=true 则删除并重建，否则报错                        |
| source 已是正确链接          | 跳过，显示 "Already linked"                                   |
| source 已是错误链接          | 报错或使用 --force                                            |

**冲突处理方案（on_exists）：**

当 target 已存在文件或文件夹时的处理策略：

| 策略       | 说明                                           | 适用场景                               |
| ---------- | ---------------------------------------------- | -------------------------------------- |
| `skip`     | 跳过该 source，不做任何操作（默认）            | 保留 target 的现有数据，避免覆盖       |
| `merge`    | 合并源到目标后删除源目录，继续创建链接         | 以 target 为准，以 source 补充         |
| `overwrite`| 删除源后继续创建链接                          | 确认 source 数据不再需要，保留目标数据 |
| `replace`  | 删除目标，移动源到目标位置，创建链接           | 确认 target 数据不再需要，可以完全替换 |

**示例：**

```bash
# 链接所有应用
link-disk link --all

# 链接单个应用
link-disk link vscode

# 链接多个应用
link-disk link vscode chrome

# 模拟运行（不实际执行）
link-disk link vscode --dry-run

# 显示详细过程
link-disk link vscode -v

# 强制重新链接（删除已有软链接）
link-disk link vscode -f
```

---

### 3.3 unlink - 移除链接

将文件从目标位置移回源位置，删除链接。

```bash
link-disk unlink [应用名...] [选项]
```

**选项：**

| 选项 | 说明 |
|------|------|
| `--all`, `-a` | 处理所有已配置的应用 |
| `--force`, `-f` | 强制移除，跳过确认 |
| `-k, --keep-files` | 只删除链接，不移动文件 |

**示例：**

```bash
# 移除所有链接（需要确认）
link-disk unlink --all

# 移除单个应用
link-disk unlink vscode

# 强制移除（跳过确认）
link-disk unlink vscode --force

# 只删除链接，保留目标位置的文件
link-disk unlink vscode --force --keep-files
```

**注意：** `unlink` 会：

1. 删除之前创建的符号链接
2. 将文件从目标位置移回源位置（除非使用 `--keep-files`）

---

### 3.4 list - 列出链接

列出所有已配置的应用和它们的链接配置。

```bash
link-disk list [选项]
```

**选项：**

| 选项 | 说明 |
|------|------|
| `--app <应用名>` | 只显示指定应用 |

**示例输出：**

```
App: VSCode
  <home>/AppData/Roaming/Code -> vscode/Roaming
  <home>/.vscode -> vscode/config

App: Chrome
  <home>/AppData/Local/Google/Chrome -> chrome/Local
```

---

### 3.5 status - 检查链接状态

检查链接是否正常工作。

```bash
link-disk status [应用名...]
```

**状态说明：**

| 状态 | 图标 | 说明 |
|------|------|------|
| `linked` | ✓ | 链接有效，目标存在 |
| `broken` | ✗ | 链接存在但目标不存在 |
| `both_exist` | ? | 源和目标都存在（非链接） |
| `source_only` | ? | 只有源存在 |
| `target_only` | ? | 只有目标存在 |
| `none` | ? | 都不存在 |

**示例：**

```bash
# 检查所有链接
link-disk status

# 检查指定应用
link-disk status vscode chrome
```

---

### 3.6 repair - 修复链接

修复损坏的链接或为孤立的目标创建新链接。

```bash
link-disk repair [应用名...] [选项]
```

**选项：**

| 选项 | 说明 |
|------|------|
| `--force`, `-f` | 强制修复（为孤立目标创建链接） |

**示例：**

```bash
# 修复所有损坏的链接
link-disk repair

# 修复指定应用
link-disk repair vscode

# 强制修复（包括为孤立目标创建链接）
link-disk repair --force
```

---

## 4. 全局选项

以下选项可用于所有命令：

| 选项 | 说明 |
|------|------|
| `--verbose`, `-v` | 详细输出模式 |
| `--config <PATH>` | 指定配置文件路径（默认 `~/.link-disk/config.toml`） |

**示例：**

```bash
# 使用自定义配置文件
link-disk --config "E:/my-config.toml" link vscode

# 详细模式
link-disk -v status
```

---

## 5. 配置文件

详细说明请参考 [配置文件说明](config.md)。

### 5.1 快速配置

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
```

### 5.2 配置位置

默认配置文件位置：

| 操作系统    | 路径                                       |
| ----------- | ------------------------------------------ |
| Windows     | `C:\Users\<用户名>\.link-disk\config.toml` |
| Linux/macOS | `/home/<用户名>/.link-disk/config.toml`    |

---

## 6. 常见问题

### 6.1 硬链接 vs 软链接，该选哪个？

**软链接（symlink）：**

- ✅ 支持跨分区
- ✅ 目标删除后链接"断开"（可用 repair 修复）
- ⚠️ 需要系统权限（Windows）
- 推荐日常使用

**硬链接（hardlink）：**

- ❌ 不能跨分区
- ✅ 不需要特殊权限
- ✅ 文件删除不受影响
- 仅同分区使用

**推荐：** 大多数情况下使用软链接（`symlink`）

---

### 6.2 链接损坏怎么办？

1. 使用 `status` 命令检查损坏情况：

   ```bash
   link-disk status
   ```

2. 使用 `repair` 命令尝试修复：

   ```bash
   link-disk repair
   ```

3. 如果修复失败，可以手动：
   - 删除损坏的链接
   - 从目标位置复制/移动文件回源位置
   - 重新运行 `link`

---

### 6.3 如何转移新的应用？

1. 编辑配置文件，添加新应用：

   ```toml
   [apps.newapp]
   name = "New App"
   on_exists = "skip"

   [[apps.newapp.sources]]
   source = "<home>/AppData/Local/NewApp"
   target = "newapp/Local"
   ```

2. 运行 link 命令：
   ```bash
   link-disk link newapp
   ```

---

### 6.4 卸载/移除链接后软件还能用吗？

可以。`unlink` 命令会：

1. 删除链接
2. 将文件从目标位置移回源位置

软件恢复原状，可以正常使用。

---

### 6.5 链接操作需要管理员权限吗？

在 Windows 上：

- 创建软链接通常需要管理员权限或启用开发者模式
- 硬链接不需要特殊权限

**解决方案：**

1. 以管理员身份运行命令提示符/PowerShell
2. 启用 Windows 开发者模式（设置 → 隐私和安全 → 开发者选项）

---

### 6.6 符号链接删除失败怎么办？

如果遇到 "Failed to remove symlink" 错误：

1. **检查是否是目录符号链接**: Windows 上目录符号链接需要使用 `remove_dir` 而非 `remove_file`
2. **检查权限**: 可能需要管理员权限
3. **检查链接状态**: 使用 `link-disk status -v` 查看详细信息

当前版本已自动处理此问题，会先尝试 remove_dir 再尝试 remove_file。

---

## 7. 故障排除

### 7.1 错误：权限被拒绝

**问题：** 无法创建符号链接

**解决方案：**

- 以管理员身份运行
- 或启用 Windows 开发者模式

### 7.2 错误：目标路径不存在

**问题：** 配置的 target 路径不存在

**解决方案：**

- 检查 workspace.path 配置是否正确
- 确保目标磁盘有足够空间

### 7.3 错误：源路径不存在

**问题：** 指定的源文件夹不存在

**解决方案：**

- 检查配置中的 source 路径是否正确
- 确认应用已安装并运行过

### 7.4 链接显示正常但软件异常

**问题：** `status` 显示正常但软件无法启动

**解决方案：**

- 检查目标文件夹权限
- 某些软件对文件位置有严格要求，尝试 `unlink` 恢复

### 7.5 占位符没有展开

**问题：** 配置中的 `<home>` 等占位符没有被替换

**解决方案：**

- 检查占位符拼写是否正确
- 使用 `link-disk link -v` 查看展开后的实际路径
- 确保 dirs crate 正常工作（系统环境变量正确）

---

## 8. 命令行帮助

查看完整帮助：

```bash
# 主帮助
link-disk --help

# 子命令帮助
link-disk init --help
link-disk link --help
link-disk unlink --help
link-disk list --help
link-disk status --help
link-disk repair --help
```
