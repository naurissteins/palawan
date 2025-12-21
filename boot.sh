#!/bin/bash

ansi_art='                 ▄▄▄                                                   
██████╗  █████╗ ██╗      █████╗ ██╗    ██╗ █████╗ ███╗   ██╗
██╔══██╗██╔══██╗██║     ██╔══██╗██║    ██║██╔══██╗████╗  ██║
██████╔╝███████║██║     ███████║██║ █╗ ██║███████║██╔██╗ ██║
██╔═══╝ ██╔══██║██║     ██╔══██║██║███╗██║██╔══██║██║╚██╗██║
██║     ██║  ██║███████╗██║  ██║╚███╔███╔╝██║  ██║██║ ╚████║
╚═╝     ╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝ ╚══╝╚══╝ ╚═╝  ╚═╝╚═╝  ╚═══╝
'

clear
echo -e "\n$ansi_art\n"

sudo pacman -Syu --noconfirm --needed git

# Use custom repo if specified, otherwise default to balabac/palawan
PALAWAN_REPO="${PALAWAN_REPO:-balabac/palawan}"

echo -e "\nCloning Palawan from: https://github.com/${PALAWAN_REPO}.git"
rm -rf ~/.local/share/palawan/
git clone "https://github.com/${PALAWAN_REPO}.git" ~/.local/share/palawan >/dev/null

# Use custom branch if instructed, otherwise default to master
PALAWAN_REF="${PALAWAN_REF:-master}"
if [[ $PALAWAN_REF != "master" ]]; then
  echo -e "\e[32mUsing branch: $PALAWAN_REF\e[0m"
  cd ~/.local/share/palawan
  git fetch origin "${PALAWAN_REF}" && git checkout "${PALAWAN_REF}"
  cd -
fi

echo -e "\nInstallation starting..."
source ~/.local/share/palawan/install.sh