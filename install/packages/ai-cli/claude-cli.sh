install_claude_cli() {
    print_section "=== Installing Claude CLI ==="

    if [ -s /usr/share/nvm/init-nvm.sh ]; then
        # Ensure npm is on PATH for non-interactive shells.
        source /usr/share/nvm/init-nvm.sh
    fi

    if ! command -v npm >/dev/null 2>&1; then
        echo "npm is not available; install Node.js first (e.g. run: nvm install --lts)."
        return 1
    fi

    npm install -g @anthropic-ai/claude-code --loglevel=info --progress=true
}

if ask_yes_no "Install Claude CLI?"; then
    echo "-> Installing Claude CLI..."
    install_claude_cli
fi
