install_nodejs() {
    print_section "=== Installing nvm (Node Version Manager) via yay ==="
    yay -S --noconfirm nvm

    BASHRC_FILE="$HOME/.bashrc"
    NVM_INIT_LINE="source /usr/share/nvm/init-nvm.sh"

    print_section "=== Configuring shell for nvm ==="
    if [ ! -f "$BASHRC_FILE" ]; then
        echo "Creating $BASHRC_FILE..."
        touch "$BASHRC_FILE"
    fi

    if ! grep -qF "$NVM_INIT_LINE" "$BASHRC_FILE"; then
        echo "Adding nvm source to $BASHRC_FILE"
        echo -e "\n# Load nvm\n$NVM_INIT_LINE" >> "$BASHRC_FILE"
    else
        echo "nvm source already in $BASHRC_FILE."
    fi

    echo "Sourcing nvm for current session to install Node.js..."
    source /usr/share/nvm/init-nvm.sh

    echo "Installing latest LTS version of Node.js..."
    nvm install --lts
    nvm use --lts
    corepack enable
    corepack prepare pnpm@latest --activate

    echo "Node.js LTS installation complete."
}

if ask_yes_no "Install Node.js (via nvm)?"; then
    echo "-> Installing Node.js..."
    install_nodejs
fi