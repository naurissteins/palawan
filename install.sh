#!/bin/bash

# Exit immediately if a command exits with a non-zero status
set -eEo pipefail

# Define Palawan locations
export PALAWAN_PATH="$HOME/.local/share/palawan"
export PALAWAN_INSTALL="$PALAWAN_PATH/install"
export PALAWAN_INSTALL_LOG_FILE="$PALAWAN_PATH/install.log"
export PATH="$PALAWAN_PATH/bin:$PATH"

# Install via Python (shell bootstrap stays minimal)
if ! command -v python3 >/dev/null 2>&1; then
  echo "python3 is required to run the installer."
  exit 1
fi

python3 "$PALAWAN_PATH/install.py" "$@"
