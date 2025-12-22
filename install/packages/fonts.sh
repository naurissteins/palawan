install_fonts() {
    print_section "=== Installing Fonts ==="
    sudo pacman -S --needed --noconfirm \
      ttf-jetbrains-mono \
      ttf-jetbrains-mono-nerd \
      ttf-cascadia-code-nerd \
      ttf-cascadia-mono-nerd \
      noto-fonts
}

if [ "${PALAWAN_INSTALL_FONTS:-0}" = "1" ]; then
    echo "-> Installing Fonts..."
    install_fonts
else
    echo "-> Skipping Fonts..."
fi
