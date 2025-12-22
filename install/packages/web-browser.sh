install_web_browsers() {
    local -a pacman_packages=()
    local -a yay_packages=()

    init_prompt_colors
    echo "Select browsers to install (space-separated, blank to skip):"
    echo "  1) Firefox"
    echo "  2) Chromium"
    echo "  3) Ungoogled Chromium (AUR)"
    echo "  4) Brave (AUR)"
    echo "  5) Zen Browser (AUR)"
    echo "  6) LibreWolf (AUR)"
    read -p "${PROMPT_COLOR}Enter choices [1 2 3 4 5 6]: ${RESET_COLOR}" choices </dev/tty

    if [ -z "${choices// }" ]; then
        echo "-> Skipping browser install."
        return 0
    fi

    for choice in $choices; do
        case "$choice" in
            1|firefox|f) pacman_packages+=(firefox firefox-ublock-origin);;
            2|chromium|c) pacman_packages+=(chromium);;
            3|ungoogled-chromium-bin|u) yay_packages+=(ungoogled-chromium-bin);;
            4|brave-bin|b) yay_packages+=(brave-bin);;
            5|zen-browser-bin|z) yay_packages+=(zen-browser-bin);;
            6|librewolf-bin|l) yay_packages+=(librewolf-bin);;
            *) echo "Unknown option: $choice";;
        esac
    done

    if [ ${#pacman_packages[@]} -eq 0 ] && [ ${#yay_packages[@]} -eq 0 ]; then
        echo "-> Skipping browser install."
        return 0
    fi

    print_section "=== Installing Web Browsers ==="
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

install_web_browsers
