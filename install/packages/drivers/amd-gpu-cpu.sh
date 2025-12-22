install_amd() {
    print_section "=== Installing AMD GPU & CPU drivers ==="
    sudo pacman -S --needed --noconfirm \
      mesa \
      vulkan-radeon \
      xf86-video-amdgpu \
      mesa-utils
}

if [ "${PALAWAN_INSTALL_AMD_DRIVERS:-0}" = "1" ]; then
    echo "-> Installing AMD drivers..."
    install_amd
else
    echo "-> Skipping AMD drivers..."
fi
