# Bridgent

无状态、零配置的 CLI 工具：把 agent 配置目录（`.pi`、`.codex`、`.claude`…）软连接到任意托管位置，并写入 `.git/info/exclude`，避免仓库因成员使用不同 agent 而被污染。

## 安装

**macOS / Linux 一行命令安装**（自动检测系统与架构，从 GitHub Releases 下载最新版）：

```bash
curl -fsSL https://raw.githubusercontent.com/liuenzuo666/bridgent/main/install.sh | bash
```

默认安装到 `~/.local/bin`，也可自定义目录（需对应写权限）：

```bash
INSTALL_DIR=/usr/local/bin curl -fsSL https://raw.githubusercontent.com/liuenzuo666/bridgent/main/install.sh | bash
```

支持的平台：macOS（Apple Silicon）、Linux（x86_64）。

> 需要源码安装（如平台暂无预编译包）时，可用 `cargo install --path .`（或 `cargo install --git https://github.com/liuenzuo666/bridgent`）。

## 用法

### 接管已有配置目录

```bash
bridgent link .pi --to .bridgent/pi
```

迁移数据 → 建软链 → 写入 exclude。之后 `git add .bridgent` 即可共享团队规范。

### clone 后恢复

```bash
bridgent link .pi --to .bridgent/pi
bridgent link .codex --to .bridgent/codex
```

### 文件型配置（如 copilot 指令）

```bash
bridgent link .github/copilot-instructions.md --to .bridgent/copilot.md --kind file
```

### 托管到仓库之外

```bash
bridgent link .codex --to ~/agents/my-project/codex
```

### 移除链接（数据保留）

```bash
bridgent unlink .pi          # 数据保留在 target
bridgent unlink .pi --purge  # 同时删除 target 数据
```

### 查看状态

```bash
bridgent status              # 扫描仓库根下所有软链
bridgent status .pi --json   # 指定路径，JSON 输出
```

### 环境检查

```bash
bridgent doctor
```

## 常用选项

| 选项 | 说明 |
|---|---|
| `--to <target>` | 托管位置；相对路径以仓库根为基准，支持绝对路径、`~`、`../` |
| `--kind <dir\|file>` | 需新建 target 时的类型（默认 `dir`） |
| `--force` | 覆盖指向其他位置的已有软链 |
| `--no-exclude` | 不写 exclude（非 git 仓库用） |
| `--yes` | 跳过交互确认 |
| `--dry-run` | 预览不落盘 |
| `--project-dir <dir>` | 指定仓库根 |
