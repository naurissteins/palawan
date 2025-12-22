install_codex_cli() {
    print_section "=== Installing Codex CLI ==="

    if [ -s /usr/share/nvm/init-nvm.sh ]; then
        # Ensure npm is on PATH for non-interactive shells.
        source /usr/share/nvm/init-nvm.sh
    fi

    if ! command -v npm >/dev/null 2>&1; then
        echo "npm is not available; install Node.js first (e.g. run: nvm install --lts)."
        return 1
    fi

    npm install -g @openai/codex --loglevel=info --progress=true
}

if [ "${PALAWAN_INSTALL_CODEX_CLI:-0}" = "1" ]; then
    echo "-> Installing Codex CLI..."
    install_codex_cli
else
    echo "-> Skipping Codex CLI..."
fi
