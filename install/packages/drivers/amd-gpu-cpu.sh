install_amd() {
    echo "=== Installing AMD GPU & CPU drivers ==="
    sudo pacman -S --needed --noconfirm \
      mesa \
      vulkan-radeon \
      xf86-video-amdgpu \
      mesa-utils
}

if ask_yes_no "Install AMD drivers?"; then
    echo "-> Installing AMD drivers..."
    install_amd
fi