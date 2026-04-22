# link-disk 使用手册

## 1. 安装

### 1.1 从源码编译

```bash
git clone https://github.com/yourname/link-disk.git
cd link-disk
cargo build --release
# 可执行文件会在 target/release/link-disk.exe
```

### 1.2 添加到 PATH

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
link-disk list

# 查看详细信息
link-disk status
```

---

## 3. 命令详解

### 3.1 init - 初始化工作区

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

```bash
link-disk link [应用名...] [选项]
```

**参数：**
- 不指定应用名：链接所有已配置的应用
- 指定应用名：只链接指定的应用

**选项：**
| 选项 | 说明 |
|------|------|
| `--dry-run` | 模拟运行，不实际执行操作 |
| `--verbose` | 显示详细信息 |

**link 逻辑说明：**

| 情况 | 处理方式 |
|------|----------|
| source 存在，target 不存在 | 移动 source → target，创建链接 |
| source 存在，target 存在 | 根据 on_exists 处理（skip/merge/replace），然后移动并创建链接 |
| source 不存在，target 不存在 | 创建 target 目录，创建链接 |
| source 不存在，target 存在 | 直接创建链接 |
| source 已是正确链接 | 跳过，显示 "Already linked" |
| source 已是错误链接 | 报错 |

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
link-disk link vscode --verbose
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
| `--force` | 强制移除，不确认 |
| `-k, --keep-files` | 只删除链接，不移动文件 |

**示例：**
```bash
# 移除所有链接
link-disk unlink --all

# 移除单个应用
link-disk unlink vscode

# 强制移除（跳过确认）
link-disk unlink vscode --force

# 只删除链接，保留目标位置的文件
link-disk unlink vscode --force --keep-files
```

**注意：** `unlink` 会：
1. 删除之前创建的链接
2. 将文件从目标位置移回源位置（除非使用 `--keep-files`）

---

### 3.4 list - 列出链接

```bash
link-disk list [选项]
```

**选项：**
| 选项 | 说明 |
|------|------|
| `--app <应用名>` | 只显示指定应用 |

**示例输出：**
```
应用: VSCode
  ✓ AppData/Roaming/Code → vscode/Roaming (软链接)
  ✓ .vscode → vscode/config (软链接)

应用: Chrome
  ✓ AppData/Local/Google/Chrome → chrome/Local (软链接)

应用: WeChat DevTools
  ✗ Navigator/Cache → wx-devtools/Cache (链接已损坏)
```

---

### 3.5 status - 检查链接状态

```bash
link-disk status [应用名...]
```

**状态说明：**
| 状态 | 说明 |
|------|------|
| `✓ 正常` | 链接有效，文件存在 |
| `✗ 损坏` | 链接存在但目标文件不存在 |
| `? 孤立` | 文件存在但链接不存在 |
| `- 未链接` | 尚未创建链接 |

**示例：**
```bash
# 检查所有链接
link-disk status

# 检查指定应用
link-disk status vscode chrome
```

---

### 3.6 repair - 修复链接

```bash
link-disk repair [应用名...] [选项]
```

**选项：**
| 选项 | 说明 |
|------|------|
| `--force` | 强制修复（删除损坏的链接重新创建） |

**示例：**
```bash
# 修复所有损坏的链接
link-disk repair

# 修复指定应用
link-disk repair vscode
```

---

## 4. 配置文件

详细说明请参考 [配置文件说明](config.md)。

### 4.1 快速配置

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

### 4.2 配置位置

默认配置文件位置：

| 操作系统 | 路径 |
|---------|------|
| Windows | `C:\Users\<用户名>\.link-disk\config.toml` |
| Linux/macOS | `/home/<用户名>/.link-disk/config.toml` |

指定自定义配置文件：
```bash
link-disk --config "E:/my-config.toml" link vscode
```

---

## 5. 常见问题

### 5.1 硬链接 vs 软链接，该选哪个？

**软链接（symlink）：**
- ✅ 支持跨分区
- ✅ 目标删除后链接"断开"
- ⚠️ 需要系统权限
- 推荐日常使用

**硬链接（hardlink）：**
- ❌ 不能跨分区
- ✅ 不需要特殊权限
- ✅ 文件删除不受影响
- 仅同分区使用

**推荐：** 大多数情况下使用软链接（`symlink`）

---

### 5.2 链接损坏怎么办？

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

### 5.3 如何转移新的应用？

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

### 5.4 卸载/移除链接后软件还能用吗？

可以。`unlink` 命令会：
1. 删除链接
2. 将文件从目标位置移回源位置

软件恢复原状，可以正常使用。

---

### 5.5 链接操作需要管理员权限吗？

在 Windows 上：
- 创建软链接通常需要管理员权限或启用开发者模式
- 硬链接不需要特殊权限

**解决方案：**
1. 以管理员身份运行命令提示符/PowerShell
2. 启用 Windows 开发者模式（设置 → 隐私和安全 → 开发者选项）

---

## 6. 故障排除

### 6.1 错误：权限被拒绝

**问题：** 无法创建符号链接

**解决方案：**
- 以管理员身份运行
- 或启用 Windows 开发者模式

### 6.2 错误：目标路径不存在

**问题：** 配置的 target 路径不存在

**解决方案：**
- 检查 workspace.path 配置是否正确
- 确保目标磁盘有足够空间

### 6.3 错误：源路径不存在

**问题：** 指定的源文件夹不存在

**解决方案：**
- 检查配置中的 source 路径是否正确
- 确认应用已安装并运行过

### 6.4 链接显示正常但软件异常

**问题：** `status` 显示正常但软件无法启动

**解决方案：**
- 检查目标文件夹权限
- 某些软件对文件位置有严格要求，尝试 `unlink` 恢复

---

## 7. 命令行帮助

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
