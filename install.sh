#!/usr/bin/env bash
# redial 一键安装脚本
# 用法: curl -fsSL https://raw.githubusercontent.com/zzfn/server/main/install.sh | bash

set -euo pipefail

REPO="zzfn/server"
BIN_NAME="redial"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Linux)
    case "$ARCH" in
      x86_64)  ARTIFACT="redial-linux-x86_64" ;;
      aarch64|arm64) ARTIFACT="redial-linux-aarch64" ;;
      *) echo "不支持的架构: $ARCH" >&2; exit 1 ;;
    esac
    ;;
  Darwin)
    case "$ARCH" in
      x86_64) ARTIFACT="redial-macos-x86_64" ;;
      arm64)  ARTIFACT="redial-macos-aarch64" ;;
      *) echo "不支持的架构: $ARCH" >&2; exit 1 ;;
    esac
    ;;
  *)
    echo "不支持的系统: $OS" >&2; exit 1 ;;
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
echo "运行以下命令启动（首次自动引导配置）:"
echo ""
echo "  $BIN_NAME"
echo ""
