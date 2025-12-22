#!/bin/bash

# Exit immediately if a command exits with a non-zero status
set -eEo pipefail

# Define Palawan locations
export PALAWAN_PATH="$HOME/.local/share/palawan"
export PALAWAN_INSTALL="$PALAWAN_PATH/install"
export PALAWAN_INSTALL_LOG_FILE="$PALAWAN_PATH/install.log"
export PATH="$PALAWAN_PATH/bin:$PATH"

# Install via Go (shell bootstrap stays minimal)
if ! command -v go >/dev/null 2>&1; then
  echo "go is required to run the installer."
  exit 1
fi

mkdir -p "$PALAWAN_PATH/bin"
(cd "$PALAWAN_PATH" && go mod download)
(cd "$PALAWAN_PATH" && go build -o "$PALAWAN_PATH/bin/palawan-installer" "./cmd/installer")
"$PALAWAN_PATH/bin/palawan-installer" "$@"
