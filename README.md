Install (bootstrapped)
```
curl -fsSL https://naurissteins.com/install | sh
```

Requirements
- Arch Linux
- `sudo` configured for your user
- Network access to GitHub for the release binary download
- Rust toolchain (for local builds)

Local build
```
cargo build --release
```

Run locally
```
./target/release/palawan-installer
```

Release notes
- Binary name must be `palawan-installer` to match `boot.sh`.
- The installer uses `sudo pacman` to install base packages.
- Base packages live in `packages/base.txt`.
  - The list is embedded into the binary at build time.
  - Override at runtime with `PALAWAN_PACKAGES_FILE=/path/to/list.txt`.
  - Or pass `--packages-file /path/to/list.txt`.
