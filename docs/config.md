# link-disk 配置文件说明

## 1. 配置文件位置

默认配置文件位置：`~/.link-disk/config.toml`

| 操作系统 | 路径 |
|---------|------|
| Windows | `C:\Users\<用户名>\.link-disk\config.toml` |
| Linux/macOS | `/home/<用户名>/.link-disk/config.toml` |

---

## 2. 完整配置示例

```toml
# link-disk 配置文件
# 用途：将软件数据从默认位置转移到其他磁盘

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
on_exists = "merge"

[[apps.chrome.sources]]
source = "<home>/AppData/Local/Google/Chrome"
target = "chrome/Local"
link_type = "symlink"

[apps.navigator]
name = "Ali-Navigator"
on_exists = "skip"

[[apps.navigator.sources]]
source = "<home>/AppData/Local/Packages/ChinaUnionTech.Navigator"
target = "navigator/Package"
link_type = "symlink"
```

---

## 3. 配置项详解

### 3.1 [workspace] 工作区配置

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `path` | 字符串 | ✅ | 工作区根路径，所有转移的文件都放在这里 |

**示例：**
```toml
[workspace]
path = "D:/link-disk-workspace"
```

### 3.2 [apps.应用名] 应用配置

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `name` | 字符串 | ✅ | 应用显示名称 |
| `enabled` | 布尔 | ❌ | 是否启用，默认 true |
| `on_exists` | 枚举 | ❌ | 目标已存在的处理策略，默认 `skip` |

**on_exists 可选值：**

| 值 | 说明 |
|----|------|
| `skip` | 如果目标已存在，跳过此操作（默认） |
| `merge` | 合并源内容到目标，删除源目录，直接创建链接 |
| `overwrite` | 删除源，保留目标，直接创建链接 |
| `replace` | 删除目标，移动源到目标位置，创建链接 |

### 3.3 [[apps.应用名.sources]] 源配置

每个应用可以配置多个 source，每个 source 代表一个需要转移的文件夹。

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `source` | 字符串 | ✅ | 源文件夹路径，支持占位符 |
| `target` | 字符串 | ✅ | 目标子路径（相对于 workspace.path，实际会拼接 `{app_name}/{target}`） |
| `link_type` | 枚举 | ❌ | 链接类型，默认 `symlink` |
| `_source_type` | 字符串 | ❌ | 源类型标识，默认 `dir`（预留字段） |

**link_type 可选值：**

| 值 | 说明 | 跨分区支持 |
|----|------|-----------|
| `symlink` | 软链接/符号链接 | ✅ 支持 |
| `hardlink` | 硬链接 | ❌ 仅同分区 |

---

## 4. 环境变量/占位符

配置文件中可以使用以下占位符，程序会自动替换为实际路径：

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

**使用示例：**

```toml
# 转移 VSCode 配置
source = "<home>/AppData/Roaming/Code"
# 解析后: C:/Users/YourName/AppData/Roaming/Code

# 转移浏览器数据
source = "<localappdata>/Google/Chrome"
# 解析后: C:/Users/YourName/AppData/Local/Google/Chrome
```

---

## 5. 路径解析示例

### 5.1 源路径解析

> **注意:** target 实际解析时会自动拼接应用名称前缀，即 `{workspace.path}/{app_name}/{target}`

配置：
```toml
source = "<home>/AppData/Roaming/Code"
target = "vscode/Roaming"
workspace.path = "D:/link-disk-workspace"
app name = "VSCode"
```

解析结果：
```
源路径: C:\Users\YourName\AppData\Roaming\Code
目标路径: D:\link-disk-workspace\VSCode\vscode\Roaming
```

### 5.2 多源配置解析

```toml
[apps.example]
name = "Example App"

[[apps.example.sources]]
source = "<home>/Documents/Example"
target = "example/Documents"

[[apps.example.sources]]
source = "<home>/AppData/Local/Example"
target = "example/Local"
```

解析结果（假设 app name = "Example App"）：
```
源1: C:\Users\YourName\Documents\Example
目标1: D:\link-disk-workspace\Example App\example\Documents

源2: C:\Users\YourName\AppData\Local\Example
目标2: D:\link-disk-workspace\Example App\example\Local
```

---

## 6. 常见应用配置参考

### 6.1 VSCode

```toml
[apps.vscode]
name = "VSCode"
on_exists = "skip"

[[apps.vscode.sources]]
source = "<home>/AppData/Roaming/Code"
target = "vscode/Roaming"
link_type = "symlink"

[[apps.vscode.sources]]
source = "<home>/.vscode"
target = "vscode/config"
link_type = "symlink"
```

### 6.2 Chrome

```toml
[apps.chrome]
name = "Chrome"
on_exists = "skip"

[[apps.chrome.sources]]
source = "<home>/AppData/Local/Google/Chrome"
target = "chrome/Local"
link_type = "symlink"

[[apps.chrome.sources]]
source = "<home>/AppData/Roaming/Google/Chrome"
target = "chrome/Roaming"
link_type = "symlink"
```

### 6.3 微信开发者工具

```toml
[apps.wx-devtools]
name = "WeChat DevTools"
on_exists = "skip"

[[apps.wx-devtools.sources]]
source = "<localappdata>/Tencent/WeChatDevTools"
target = "wx-devtools/Cache"
link_type = "symlink"
```

---

## 7. 注意事项

### 7.1 路径分隔符

- Windows: 使用 `/` 或 `\` 均可，程序会自动处理
- Linux/macOS: 使用 `/`

### 7.2 空格和特殊字符

如果路径中包含空格或特殊字符，不需要额外引号：
```toml
source = "<home>/AppData/Roaming/Microsoft/Teams"
```

### 7.3 相对路径

target 路径是相对于 workspace.path 的相对路径，实际会拼接为 `{workspace.path}/{app_name}/{target}`：
```toml
workspace.path = "D:/link-disk-workspace"
app name = "VSCode"
target = "vscode/Roaming"  # → D:\link-disk-workspace\VSCode\vscode\Roaming
```

### 7.4 Windows 权限问题

创建符号链接在 Windows 上需要管理员权限或启用开发者模式。建议：
- 以管理员身份运行终端，或启用 Windows 开发者模式
- 使用硬链接（不需要特殊权限，但仅限同分区）
- 转移目标优先选择用户可写的文件夹

### 7.5 应用名称与路径拼接

代码中 target 实际路径为 `{workspace.path}/{app_config.name}/{target}`，其中 `app_config.name` 是配置中 `name` 字段的值（非 `[apps.xxx]` 中的键名）。配置 target 时请注意不要与 name 产生冗余路径。
