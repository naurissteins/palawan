Install (bootstrapped)
```
curl -fsSL https://naurissteins.com/install | sh
```

Requirements
- Arch Linux
- `sudo` configured for your user
- Network access to GitHub for the release binary download

Local build
```
go mod download
go build -o palawan-installer .
```

Run locally
```
./palawan-installer
```

Release notes
- Binary name must be `palawan-installer` to match `boot.sh`.
- The installer uses `sudo pacman` to install base packages.
