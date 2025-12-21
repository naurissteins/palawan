#!/bin/bash

# Exit immediately if a command exits with a non-zero status
set -eEo pipefail

# Define Palawan locations
export PALAWAN_PATH="$HOME/.local/share/palawan"
export PALAWAN_INSTALL="$PALAWAN_PATH/install"
export PALAWAN_INSTALL_LOG_FILE="/var/log/palawan-install.log"
export PATH="$PALAWAN_PATH/bin:$PATH"

# Install
source "$PALAWAN_INSTALL/helpers/all.sh"
source "$PALAWAN_INSTALL/preflight/all.sh"
source "$PALAWAN_INSTALL/packaging/all.sh"
source "$PALAWAN_INSTALL/config/all.sh"
source "$PALAWAN_INSTALL/login/all.sh"
source "$PALAWAN_INSTALL/post-install/all.sh"