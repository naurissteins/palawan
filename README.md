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
  - Hyprland packages live in `packages/hyprland.txt` and install in a dedicated step.
  - The list is embedded into the binary at build time.
  - Override at runtime with `PALAWAN_PACKAGES_FILE=/path/to/list.txt`.
  - Or pass `--packages-file /path/to/list.txt`.

Code layout
- `src/main.rs` wires modules together and runs the main TUI loop.
- `src/installer.rs` handles install steps, sudo, and command execution.
- `src/drivers.rs` detects GPU vendors and selects driver packages.
- `src/ui.rs` renders the installer UI and selection screens.
- `src/selection.rs` defines selectable browser/terminal/editor choices and selection logic.
- `src/model.rs` contains shared app state and event types.
- `src/packages.rs` loads package lists and parses CLI args.

Adding a new chooser step
- Add choices and selection logic in `src/selection.rs` (similar to `BROWSER_CHOICES`).
- Add a TUI selector in `src/ui.rs` and return a `PackageSelection`.
- Extend `STEP_NAMES` and install logic in `src/installer.rs`.
- Wire the new selection into `src/main.rs` before starting the installer thread.

Development overrides
- `PALAWAN_DEV_GPU=amd|intel|nvidia` (comma-separated supported) forces GPU detection for testing.
  - Example: `PALAWAN_DEV_GPU=nvidia,intel ./target/release/palawan-installer`
