schedule_gnome_dark_theme() {
    print_section "=== Scheduling GTK Dark Theme ==="

    local autostart_dir="$HOME/.config/autostart"
    local autostart_file="$autostart_dir/palawan-theme.desktop"
    local desktop_template="$PALAWAN_INSTALL/post-install/assets/palawan-theme.desktop"
    local theme_script="$PALAWAN_INSTALL/post-install/run-gnome-theme.sh"
    local hypr_dir="$HOME/.config/hypr"
    local hypr_include="$hypr_dir/palawan.conf"
    local hypr_main="$hypr_dir/hyprland.conf"
    local hypr_source_line="source = $hypr_include"
    local hypr_exec_line='exec-once = /bin/bash -lc "$HOME/.local/share/palawan/install/post-install/run-gnome-theme.sh"'

    mkdir -p "$autostart_dir"
    install -m 644 "$desktop_template" "$autostart_file"
    chmod +x "$theme_script"

    if [ -d "$hypr_dir" ] || [ -f "$hypr_main" ]; then
        mkdir -p "$hypr_dir"
        printf '%s\n' "# Palawan post-install hooks" "$hypr_exec_line" >"$hypr_include"
        if [ -f "$hypr_main" ]; then
            if ! grep -Fqx "$hypr_source_line" "$hypr_main"; then
                echo "$hypr_source_line" >>"$hypr_main"
            fi
        fi
    fi
}

schedule_gnome_dark_theme
