install_hyprland() {
    print_section "=== Installing Hyprland and core Wayland dependencies ==="
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
      hyprpaper \
      hypridle \
      hyprland-guiutils \
      hyprsunset

    print_section "=== Setting up Hyprland config ==="
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

if ask_yes_no "Install Hyprland and core Wayland dependencies?"; then
    echo "-> Installing Hyprland and core Wayland dependencies..."
    install_hyprland
fi