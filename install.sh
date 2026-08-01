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
  x86_64 | amd64)
    if [ "$OS" = "apple-darwin" ]; then
      echo "error: 暂不支持 Intel Mac (x86_64)，预编译包仅提供 Apple Silicon (arm64)" >&2
      exit 1
    fi
    ARCH="x86_64"
    ;;
  *)
    echo "error: 暂不支持架构 $(uname -m)，预编译包仅提供 arm64 / x86_64" >&2
    exit 1
    ;;
esac

TARGET="${ARCH}-${OS}"

# --- 获取最新版本（git ls-remote，避免 GitHub API 限流；可用 BRIDGENT_VERSION 指定） ---
echo "==> 获取最新版本..."
VERSION="${BRIDGENT_VERSION:-}"
if [ -z "$VERSION" ]; then
  if ! command -v git >/dev/null 2>&1; then
    echo "error: 未找到 git 命令；可手动指定版本：BRIDGENT_VERSION=v0.1.0" >&2
    exit 1
  fi
  VERSION="$(git ls-remote --tags --refs "https://github.com/${REPO}.git" 'refs/tags/v*' | awk -F/ '{print $NF}' | sort -V | tail -n1)"
fi
if [ -z "$VERSION" ]; then
  echo "error: 无法获取最新版本（git ls-remote 失败，请检查网络/代理，或手动指定版本）" >&2
  echo "  兜底: BRIDGENT_VERSION=v0.1.0 curl -fsSL https://raw.githubusercontent.com/${REPO}/main/install.sh | bash" >&2
  exit 1
fi
echo "    最新版本: $VERSION"

# --- 下载并解压 ---
URL="https://github.com/${REPO}/releases/download/${VERSION}/bridgent-${TARGET}.tar.gz"
echo "==> 下载 ${TARGET} 二进制..."
echo "    下载地址: $URL"
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
