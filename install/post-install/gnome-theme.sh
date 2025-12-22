schedule_gnome_dark_theme() {
    print_section "=== Scheduling GNOME Dark Theme ==="

    local autostart_dir="$HOME/.config/autostart"
    local autostart_file="$autostart_dir/palawan-theme.desktop"
    local desktop_template="$PALAWAN_INSTALL/post-install/assets/palawan-theme.desktop"
    local theme_script="$PALAWAN_INSTALL/post-install/run-gnome-theme.sh"

    mkdir -p "$autostart_dir"
    install -m 644 "$desktop_template" "$autostart_file"
    chmod +x "$theme_script"
}

schedule_gnome_dark_theme
