#!/usr/bin/env bash
#
# bridgent 一键安装脚本
# 自动检测系统 (macOS / Linux) 与架构，从 GitHub Releases 下载对应二进制。
# 用法: curl -fsSL https://raw.githubusercontent.com/liuenzuo666/bridgent/main/install.sh | bash
# 自定义安装目录: INSTALL_DIR=/usr/local/bin curl -fsSL ... | bash
set -euo pipefail

REPO="liuenzuo666/bridgent"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

# --- 平台检测 ---
case "$(uname -s)" in
  Darwin) OS="apple-darwin" ;;
  Linux)  OS="unknown-linux-gnu" ;;
  *)
    echo "error: 暂不支持 $(uname -s)，bridgent 仅提供 macOS / Linux 预编译包" >&2
    exit 1
    ;;
esac

case "$(uname -m)" in
  arm64 | aarch64) ARCH="aarch64" ;;
  x86_64 | amd64)  ARCH="x86_64" ;;
  *)
    echo "error: 暂不支持架构 $(uname -m)，预编译包仅提供 arm64 / x86_64" >&2
    exit 1
    ;;
esac

TARGET="${ARCH}-${OS}"

# --- 获取最新版本 ---
echo "==> 获取最新版本..."
API="https://api.github.com/repos/${REPO}/releases/latest"
VERSION="$(curl -fsSL "$API" | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -n1)"
if [ -z "$VERSION" ]; then
  echo "error: 无法获取最新版本，请确认仓库已发布 Release，或稍后重试" >&2
  exit 1
fi
echo "    最新版本: $VERSION"

# --- 下载并解压 ---
URL="https://github.com/${REPO}/releases/download/${VERSION}/bridgent-${TARGET}.tar.gz"
echo "==> 下载 ${TARGET} 二进制..."
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
curl -fsSL "$URL" -o "$TMP/bridgent.tar.gz"
tar -xzf "$TMP/bridgent.tar.gz" -C "$TMP"

# --- 安装 ---
mkdir -p "$INSTALL_DIR"
install -m 0755 "$TMP/bridgent" "$INSTALL_DIR/bridgent"
echo "==> 已安装到 $INSTALL_DIR/bridgent"

# --- PATH 提示 ---
case ":$PATH:" in
  *":$INSTALL_DIR:"*) ;;
  *)
    echo ""
    echo "提示: $INSTALL_DIR 不在 PATH 中，请先将其加入 PATH："
    echo "  echo 'export PATH=\"$INSTALL_DIR:\$PATH\"' >> ~/.zshrc"
    echo "  source ~/.zshrc"
    ;;
esac

echo "==> 安装完成，运行 bridgent --help 验证"
