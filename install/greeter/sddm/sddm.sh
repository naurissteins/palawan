#!/bin/bash

set -eEuo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
theme_source="$script_dir/sddm-jupiter-theme"
theme_target="/usr/share/sddm/themes/sddm-jupiter-theme"

sudo pacman -S --noconfirm --needed sddm
sudo systemctl enable sddm.service
echo "[Theme]
Current=sddm-jupiter-theme" | sudo tee /etc/sddm.conf >/dev/null
sudo mkdir -p /etc/sddm.conf.d
echo "[General]
InputMethod=qtvirtualkeyboard" | sudo tee /etc/sddm.conf.d/virtualkbd.conf >/dev/null

if [[ ! -d "$theme_source" ]]; then
  echo "Theme directory not found: $theme_source" >&2
  exit 1
fi

sudo mkdir -p /usr/share/sddm/themes
sudo rm -rf "$theme_target"
sudo cp -r "$theme_source" "$theme_target"
