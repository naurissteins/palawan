#!/bin/bash

set -euo pipefail

if ! command -v gsettings >/dev/null 2>&1; then
  exit 0
fi

theme_marker="$HOME/.cache/palawan-theme-applied"
if [ -f "$theme_marker" ]; then
  exit 0
fi

gsettings set org.gnome.desktop.interface color-scheme 'prefer-dark'
gsettings set org.gnome.desktop.interface gtk-theme 'Adwaita-dark'

mkdir -p "$(dirname "$theme_marker")"
touch "$theme_marker"

autostart_file="$HOME/.config/autostart/palawan-theme.desktop"
if [ -f "$autostart_file" ]; then
  rm -f "$autostart_file"
fi
