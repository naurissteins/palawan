install_codex_cli() {
    print_section "=== Installing Codex CLI ==="

    if ! command -v npm >/dev/null 2>&1; then
        echo "npm is not available; install Node.js first."
        return 1
    fi

    npm install -g @openai/codex
}

if ask_yes_no "Install Codex CLI?"; then
    echo "-> Installing Codex CLI..."
    install_codex_cli
fi
