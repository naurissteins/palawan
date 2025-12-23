#!/bin/bash
set -e

echo "Bootstrapping Palawan..."

sudo pacman -Sy --noconfirm --needed curl ca-certificates

INSTALL_DIR="$HOME/.local/bin"
BIN_NAME="palawan-installer"

mkdir -p "$INSTALL_DIR"

curl -L \
  https://github.com/balabac/palawan/releases/latest/download/$BIN_NAME \
  -o "$INSTALL_DIR/$BIN_NAME"

chmod +x "$INSTALL_DIR/$BIN_NAME"

exec "$INSTALL_DIR/$BIN_NAME"
