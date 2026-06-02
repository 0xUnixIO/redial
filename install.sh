#!/usr/bin/env bash
# boil 一键安装脚本
# 用法: curl -fsSL https://raw.githubusercontent.com/0xUnixIO/boil/main/install.sh | bash

set -euo pipefail

REPO="0xUnixIO/boil"
BIN_NAME="boil"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Linux)
    case "$ARCH" in
      x86_64)        ARTIFACT="boil-linux-x86_64" ;;
      aarch64|arm64) ARTIFACT="boil-linux-aarch64" ;;
      *) echo "不支持的架构: $ARCH" >&2; exit 1 ;;
    esac
    ;;
  *)
    echo "仅支持 Linux 系统" >&2; exit 1 ;;
esac

echo "获取最新版本..."
TAG="$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
  | grep '"tag_name"' | sed 's/.*"tag_name": *"\(.*\)".*/\1/')"

[ -z "$TAG" ] && { echo "无法获取最新版本: https://github.com/$REPO/releases" >&2; exit 1; }

echo "版本: $TAG | 平台: $OS/$ARCH"

URL="https://github.com/$REPO/releases/download/$TAG/$ARTIFACT"
TMP="$(mktemp)"
trap 'rm -f "$TMP"' EXIT

echo "下载中..."
curl -fsSL "$URL" -o "$TMP" || { echo "下载失败: $URL" >&2; exit 1; }
chmod +x "$TMP"

if [ -w "$INSTALL_DIR" ]; then
  mv "$TMP" "$INSTALL_DIR/$BIN_NAME"
else
  sudo mv "$TMP" "$INSTALL_DIR/$BIN_NAME"
fi

echo ""
echo "✅ 安装完成: $INSTALL_DIR/$BIN_NAME"
echo ""

# 已有配置则跳过向导，仅首次安装时运行
if [ ! -f "/etc/boil/config.env" ] && [ ! -f "$INSTALL_DIR/config.env" ]; then
  "$INSTALL_DIR/$BIN_NAME" setup
else
  echo "检测到已有配置，跳过配置向导"
fi

# 安装 systemd 服务（已安装则重启以加载新版本）
if command -v systemctl >/dev/null 2>&1; then
  echo ""
  if systemctl is-active --quiet boil 2>/dev/null; then
    systemctl restart boil
    echo "✅ 服务已重启（新版本生效）"
  else
    "$INSTALL_DIR/$BIN_NAME" service install
  fi
else
  echo "未检测到 systemd，手动启动："
  echo "  nohup $BIN_NAME >> bot.log 2>&1 &"
fi
