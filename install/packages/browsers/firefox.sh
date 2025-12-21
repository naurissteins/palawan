install_web_browsers() {
    local -a packages=()

    init_prompt_colors
    echo "Select browsers to install (space-separated, blank to skip):"
    echo "  1) Firefox"
    echo "  2) Chromium"
    read -p "${PROMPT_COLOR}Enter choices [1 2]: ${RESET_COLOR}" choices </dev/tty

    if [ -z "${choices// }" ]; then
        echo "-> Skipping browser install."
        return 0
    fi

    for choice in $choices; do
        case "$choice" in
            1|firefox|f) packages+=(firefox firefox-ublock-origin);;
            2|chromium|c) packages+=(chromium);;
            *) echo "Unknown option: $choice";;
        esac
    done

    if [ ${#packages[@]} -eq 0 ]; then
        echo "-> Skipping browser install."
        return 0
    fi

    print_section "=== Installing Web Browsers ==="
    sudo pacman -S --needed --noconfirm "${packages[@]}"
}

install_web_browsers
