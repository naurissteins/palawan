install_nodejs() {
    print_section "=== Installing nvm (Node Version Manager) via yay ==="
    yay -S --noconfirm nvm

    BASHRC_FILE="$HOME/.bashrc"
    ZSHRC_FILE="$HOME/.zshrc"
    NVM_INIT_LINE="source /usr/share/nvm/init-nvm.sh"

    print_section "=== Configuring shell for nvm ==="
    for RC_FILE in "$BASHRC_FILE" "$ZSHRC_FILE"; do
        if [ ! -f "$RC_FILE" ]; then
            echo "Creating $RC_FILE..."
            touch "$RC_FILE"
        fi

        if ! grep -qF "$NVM_INIT_LINE" "$RC_FILE"; then
            echo "Adding nvm source to $RC_FILE"
            echo -e "\n# Load nvm\n$NVM_INIT_LINE" >> "$RC_FILE"
        else
            echo "nvm source already in $RC_FILE."
        fi
    done

    echo "Sourcing nvm for current session to install Node.js..."
    source /usr/share/nvm/init-nvm.sh

    echo "Installing latest LTS version of Node.js..."
    nvm install --lts
    nvm use --lts
    corepack enable
    corepack prepare pnpm@latest --activate

    echo "Node.js LTS installation complete."
}

if [ "${PALAWAN_INSTALL_NODEJS:-0}" = "1" ]; then
    echo "-> Installing Node.js..."
    install_nodejs
else
    echo "-> Skipping Node.js..."
fi
