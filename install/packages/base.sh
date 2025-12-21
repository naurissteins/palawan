# Install all base packages
mapfile -t packages < <(grep -v '^#' "$PALAWAN_INSTALL/palawan-base.packages" | grep -v '^$')
sudo pacman -S --noconfirm --needed "${packages[@]}"