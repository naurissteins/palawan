#!/bin/bash

set -euo pipefail

if ! command -v gsettings >/dev/null 2>&1; then
  exit 0
fi

case "${XDG_CURRENT_DESKTOP:-}" in
  *GNOME*) ;;
  *) exit 0 ;;
esac

gsettings set org.gnome.desktop.interface color-scheme 'prefer-dark'
gsettings set org.gnome.desktop.interface gtk-theme 'Adwaita-dark'

autostart_file="$HOME/.config/autostart/palawan-theme.desktop"
if [ -f "$autostart_file" ]; then
  rm -f "$autostart_file"
fi
