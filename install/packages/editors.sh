install_editors() {
    local -a pacman_packages=()
    local -a yay_packages=()

    init_prompt_colors
    echo "Select editors to install (space-separated, blank to skip):"
    echo "  1) Zed"
    echo "  2) Cursor (AUR)"
    echo "  3) Visual Studio Code (AUR)"
    echo "  4) VSCodium (AUR)"
    echo "  5) Sublime Text 4 (AUR)"
    read -p "${PROMPT_COLOR}Enter choices [1 2 3 4 5]: ${RESET_COLOR}" choices </dev/tty

    if [ -z "${choices// }" ]; then
        echo "-> Skipping editor install."
        return 0
    fi

    for choice in $choices; do
        case "$choice" in
            1|zed|z) pacman_packages+=(zed);;
            2|cursor|cursor-bin|c) yay_packages+=(cursor-bin);;
            3|visual-studio-code-bin|vscode|v) yay_packages+=(visual-studio-code-bin);;
            4|vscodium|codium) yay_packages+=(vscodium);;
            5|sublime-text-4|sublime|s) yay_packages+=(sublime-text-4);;
            *) echo "Unknown option: $choice";;
        esac
    done

    if [ ${#pacman_packages[@]} -eq 0 ] && [ ${#yay_packages[@]} -eq 0 ]; then
        echo "-> Skipping editor install."
        return 0
    fi

    print_section "=== Installing Editors ==="
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

install_editors
