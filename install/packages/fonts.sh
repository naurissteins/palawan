install_fonts() {
    print_section "=== Installing Fonts ==="
    sudo pacman -S --needed --noconfirm \
      ttf-jetbrains-mono \
      ttf-jetbrains-mono-nerd \
      ttf-cascadia-code-nerd \
      ttf-cascadia-mono-nerd \
      noto-fonts
}

if ask_yes_no "Install Fonts?"; then
    echo "-> Installing Fonts..."
    install_fonts
fi