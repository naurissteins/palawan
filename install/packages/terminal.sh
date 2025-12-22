install_terminals() {
    local -a pacman_packages=()
    local -a yay_packages=()

    init_prompt_colors
    echo "Select terminals to install (space-separated, blank to skip):"
    echo "  1) Ghostty"
    echo "  2) Kitty"
    echo "  3) Alacritty"
    read -p "${PROMPT_COLOR}Enter choices [1 2 3]: ${RESET_COLOR}" choices </dev/tty

    if [ -z "${choices// }" ]; then
        echo "-> Skipping terminal install."
        return 0
    fi

    for choice in $choices; do
        case "$choice" in
            1|ghostty|g) pacman_packages+=(ghostty);;
            2|kitty|k) pacman_packages+=(kitty);;
            3|alacritty|a) pacman_packages+=(alacritty);;
            *) echo "Unknown option: $choice";;
        esac
    done

    if [ ${#pacman_packages[@]} -eq 0 ] && [ ${#yay_packages[@]} -eq 0 ]; then
        echo "-> Skipping terminal install."
        return 0
    fi

    print_section "=== Installing Terminals ==="
    if [ ${#pacman_packages[@]} -gt 0 ]; then
        sudo pacman -S --needed --noconfirm "${pacman_packages[@]}"
    fi
    if [ ${#yay_packages[@]} -gt 0 ]; then
        if ! command -v yay >/dev/null 2>&1; then
            echo "yay is not available; skipping AUR packages."
            return 0
        fi
        yay -S --needed --noconfirm "${yay_packages[@]}"
    fi
}

install_terminals
