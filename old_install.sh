#!/usr/bin/env bash

set -e

# Function to ask a yes/no question
ask_yes_no() {
    while true;
 do
        read -p "$1 [y/n]: " yn
        case $yn in
            [Yy]* ) return 0;; # Yes
            [Nn]* ) return 1;; # No
            * ) echo "Please answer yes or no.";;
        esac
    done
}

# --- Installation Functions ---

install_amd() {
    echo "=== Installing AMD GPU & CPU drivers ==="
    sudo pacman -S --needed --noconfirm \
      mesa \
      vulkan-radeon \
      xf86-video-amdgpu \
      mesa-utils
}

install_hyprland() {
    echo "=== Installing Hyprland and core Wayland dependencies ==="
    sudo pacman -S --needed --noconfirm \
      hyprland \
      wayland \
      wayland-protocols \
      xdg-desktop-portal \
      xdg-desktop-portal-hyprland \
      xdg-desktop-portal-gtk \
      polkit-kde-agent \
      wl-clipboard \
      network-manager-applet \
      qt5-wayland \
      qt6-wayland \
      hyprlock \
      hyprpicker \
      hyprpaper

    echo "=== Setting up Hyprland config ==="
    HYPR_DIR="$HOME/.config/hypr"
    mkdir -p "$HYPR_DIR"

    if [ ! -f "$HYPR_DIR/hyprland.conf" ]; then
      # Fallback to default config if user's doesn't exist
      if [ -f "/usr/share/hyprland/hyprland.conf" ]; then
        cp /usr/share/hyprland/hyprland.conf "$HYPR_DIR/"
      elif [ -f "/usr/share/hypr/hyprland.conf" ]; then # some distros use this path
        cp /usr/share/hypr/hyprland.conf "$HYPR_DIR/"
      fi
      echo "Default hyprland.conf copied"
    else
      echo "hyprland.conf already exists, skipping copy"
    fi
}

install_yay() {
    echo "=== Installing AUR Helper (yay) ==="
    sudo pacman -S --needed --noconfirm git base-devel
    if ! command -v yay &> /dev/null
    then
        echo "yay could not be found, installing..."
        git clone https://aur.archlinux.org/yay-bin.git /tmp/yay-bin
        (cd /tmp/yay-bin && makepkg -si --noconfirm)
        rm -rf /tmp/yay-bin
    else
        echo "yay is already installed, skipping..."
    fi
}

install_nodejs() {
    echo "=== Installing nvm (Node Version Manager) via yay ==="
    yay -S --noconfirm nvm

    BASHRC_FILE="$HOME/.bashrc"
    NVM_INIT_LINE="source /usr/share/nvm/init-nvm.sh"

    echo "=== Configuring shell for nvm ==="
    if [ ! -f "$BASHRC_FILE" ]; then
        echo "Creating $BASHRC_FILE..."
        touch "$BASHRC_FILE"
    fi

    if ! grep -qF "$NVM_INIT_LINE" "$BASHRC_FILE"; then
        echo "Adding nvm source to $BASHRC_FILE"
        echo -e "\n# Load nvm\n$NVM_INIT_LINE" >> "$BASHRC_FILE"
    else
        echo "nvm source already in $BASHRC_FILE."
    fi

    echo "Sourcing nvm for current session to install Node.js..."
    source /usr/share/nvm/init-nvm.sh

    echo "Installing latest LTS version of Node.js..."
    nvm install --lts

    echo "Node.js LTS installation complete."
}

install_fonts() {
    echo "=== Installing fonts ==="
    sudo pacman -S --needed --noconfirm \
      ttf-jetbrains-mono \
      ttf-jetbrains-mono-nerd \
      ttf-cascadia-code-nerd \
      ttf-cascadia-mono-nerd \
      ttf-font-awesome \
      noto-fonts
}

install_web_browser() {
    echo "=== Installing Firefox ==="
    sudo pacman -S --needed --noconfirm \
      firefox \
      firefox-ublock-origin
}

install_other() {
    echo "=== Installing terminal, bar, launcher, and utilities ==="
    sudo pacman -S --needed --noconfirm \
      kitty \
      waybar \
      rofi-wayland \
      dunst \
      grim \
      slurp \
      xdg-user-dirs \
      nautilus

    echo "=== Installing Apps ==="
    sudo pacman -S --needed --noconfirm \
      htop \
      vim \
      neovim

    echo "=== Updating user directories ==="
    xdg-user-dirs-update
}

# --- Main Installation ---

echo "--- Starting Installation ---"
echo "You will be asked to install different component groups."
echo

if ask_yes_no "Install AMD drivers?"; then
    echo "-> Installing AMD drivers..."
    install_amd
    echo
fi

if ask_yes_no "Install Hyprland and core components?"; then
    echo "-> Installing Hyprland..."
    install_hyprland
    echo
fi

if ask_yes_no "Install AUR Helper (yay)?"; then
    echo "-> Installing yay..."
    install_yay
    echo
fi

if ask_yes_no "Install Node.js (via nvm)?"; then
    echo "-> Installing Node.js..."
    install_nodejs
    echo
fi

if ask_yes_no "Install Fonts?"; then
    echo "-> Installing fonts..."
    install_fonts
    echo
fi

if ask_yes_no "Install Web Browser (Firefox)?"; then
    echo "-> Installing web browser..."
    install_web_browser
    echo
fi

if ask_yes_no "Install other utilities (kitty, rofi, etc.) and apps?"; then
    echo "-> Installing other utilities..."
    install_other
    echo
fi

echo "=== Installation complete! Reboot recommended ==="
