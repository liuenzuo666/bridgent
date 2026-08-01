#!/usr/bin/env bash
#
# bridgent 卸载脚本
# 删除安装脚本 (install.sh) 安装的二进制。
# 用法: curl -fsSL https://raw.githubusercontent.com/liuenzuo666/bridgent/main/uninstall.sh | bash
# 自定义安装目录（须与安装时一致）: INSTALL_DIR=/usr/local/bin curl -fsSL ... | bash
set -euo pipefail

INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
BIN="$INSTALL_DIR/bridgent"

if [ ! -e "$BIN" ]; then
  echo "bridgent 未安装在 $BIN，无需卸载" >&2
  exit 0
fi

rm -f "$BIN"
echo "==> 已删除 $BIN"

# PATH 提示
case ":$PATH:" in
  *":$INSTALL_DIR:"*) ;;
  *)
    echo ""
    echo "提示: $INSTALL_DIR 仍留在 PATH 中（无害），如需清理请编辑你的 shell 配置文件"
    ;;
esac

echo "==> 卸载完成"
