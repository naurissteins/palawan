install_gemini_cli() {
    print_section "=== Installing Gemini CLI ==="

    if [ -s /usr/share/nvm/init-nvm.sh ]; then
        # Ensure npm is on PATH for non-interactive shells.
        source /usr/share/nvm/init-nvm.sh
    fi

    if ! command -v npm >/dev/null 2>&1; then
        echo "npm is not available; install Node.js first (e.g. run: nvm install --lts)."
        return 1
    fi

    npm install -g @google/gemini-cli --loglevel=info --progress=true
}

if ask_yes_no "Install Gemini CLI?"; then
    echo "-> Installing Gemini CLI..."
    install_gemini_cli
fi
