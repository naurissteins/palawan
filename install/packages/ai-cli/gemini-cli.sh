install_gemini_cli() {
    print_section "=== Installing Gemini CLI ==="

    if ! command -v npm >/dev/null 2>&1; then
        echo "npm is not available; install Node.js first."
        return 1
    fi

    npm install -g @google/gemini-cli --loglevel=info --progress=true
}

if ask_yes_no "Install Gemini CLI?"; then
    echo "-> Installing Gemini CLI..."
    install_gemini_cli
fi
