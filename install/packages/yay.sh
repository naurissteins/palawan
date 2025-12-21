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

if ask_yes_no "Install yay?"; then
    echo "-> Installing yay..."
    install_yay
fi