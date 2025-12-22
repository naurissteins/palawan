#!/usr/bin/env python3
import datetime
import os
import re
import subprocess
import sys
import time
from pathlib import Path


ANSI_RE = re.compile(r"\x1b\[[0-9;]*m")


def strip_ansi(text: str) -> str:
    return ANSI_RE.sub("", text)


def init_colors() -> dict:
    if sys.stdout.isatty():
        return {
            "green": "\033[32m",
            "red": "\033[31m",
            "reset": "\033[0m",
        }
    return {"green": "", "red": "", "reset": ""}


def log_line(log_file: Path, message: str) -> None:
    timestamp = datetime.datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    with log_file.open("a", encoding="utf-8") as handle:
        handle.write(f"[{timestamp}] {message}\n")


def start_install_log(log_file: Path, colors: dict) -> float:
    log_file.parent.mkdir(parents=True, exist_ok=True)
    log_file.write_text("", encoding="utf-8")
    start_time = time.time()
    log_line(
        log_file,
        f"=== Palawan Installation Started: {datetime.datetime.now():%Y-%m-%d %H:%M:%S} ===",
    )
    print(f"{colors['green']}Starting Palawan installation...{colors['reset']}")
    return start_time


def stop_install_log(log_file: Path, start_time: float, colors: dict) -> None:
    end_time = time.time()
    log_line(
        log_file,
        f"=== Palawan Installation Completed: {datetime.datetime.now():%Y-%m-%d %H:%M:%S} ===",
    )
    duration = int(end_time - start_time)
    with log_file.open("a", encoding="utf-8") as handle:
        handle.write("\n")
        handle.write("[Installation Time Summary]\n")
        handle.write(f"Palawan:     {duration // 60}m {duration % 60}s\n")
        handle.write("=================================\n")
    print(f"{colors['green']}Installation complete.{colors['reset']}")


def run_script(script: Path, install_dir: Path, log_file: Path, colors: dict) -> int:
    log_line(log_file, f"Starting: {script}")
    command = f"source '{install_dir / 'helpers' / 'all.sh'}'; source '{script}'"
    process = subprocess.Popen(
        ["bash", "-c", command],
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        env=os.environ.copy(),
    )

    assert process.stdout is not None
    with log_file.open("a", encoding="utf-8") as handle:
        for line in process.stdout:
            sys.stdout.write(line)
            sys.stdout.flush()
            handle.write(strip_ansi(line))

    return_code = process.wait()
    if return_code == 0:
        log_line(log_file, f"Completed: {script}")
    else:
        log_line(log_file, f"Failed: {script} (exit code: {return_code})")
        print(
            f"{colors['red']}Failed: {script} (exit code: {return_code}){colors['reset']}"
        )
    return return_code


def print_section(text: str) -> None:
    width = len(text)
    border = "-" * (width + 2)
    print(f"+{border}+")
    print(f"| {text} |")
    print(f"+{border}+")


def run_command(
    command: list[str],
    log_file: Path,
    colors: dict,
    description: str,
) -> int:
    log_line(log_file, f"Starting: {description}")
    process = subprocess.Popen(
        command,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        env=os.environ.copy(),
    )

    assert process.stdout is not None
    with log_file.open("a", encoding="utf-8") as handle:
        for line in process.stdout:
            sys.stdout.write(line)
            sys.stdout.flush()
            handle.write(strip_ansi(line))

    return_code = process.wait()
    if return_code == 0:
        log_line(log_file, f"Completed: {description}")
    else:
        log_line(log_file, f"Failed: {description} (exit code: {return_code})")
        print(
            f"{colors['red']}Failed: {description} (exit code: {return_code}){colors['reset']}"
        )
    return return_code


def install_base_packages(install_dir: Path, log_file: Path, colors: dict) -> int:
    print_section("=== Installing base packages ===")
    packages_file = install_dir / "palawan-base.packages"
    if not packages_file.exists():
        log_line(log_file, f"Missing packages file: {packages_file}")
        return 1

    packages: list[str] = []
    for line in packages_file.read_text(encoding="utf-8").splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        packages.append(stripped)

    if not packages:
        log_line(log_file, "No base packages found; skipping.")
        return 0

    command = ["sudo", "pacman", "-S", "--noconfirm", "--needed", *packages]
    return run_command(command, log_file, colors, "install base packages")


def main() -> int:
    palawan_path = Path(os.environ.get("PALAWAN_PATH", Path.home() / ".local/share/palawan"))
    install_dir = Path(os.environ.get("PALAWAN_INSTALL", palawan_path / "install"))
    log_file = Path(
        os.environ.get("PALAWAN_INSTALL_LOG_FILE", palawan_path / "install.log")
    )
    os.environ["PATH"] = f"{palawan_path / 'bin'}:{os.environ.get('PATH', '')}"

    colors = init_colors()
    start_time = start_install_log(log_file, colors)

    scripts = [
        install_dir / "packages" / "drivers" / "amd-gpu-cpu.sh",
        install_dir / "packages" / "yay.sh",
        install_dir / "packages" / "hyprland.sh",
        install_dir / "packages" / "nodejs.sh",
        install_dir / "packages" / "terminal.sh",
        install_dir / "packages" / "ai-cli" / "codex-cli.sh",
        install_dir / "packages" / "ai-cli" / "gemini-cli.sh",
        install_dir / "packages" / "ai-cli" / "claude-cli.sh",
        install_dir / "packages" / "fonts.sh",
        install_dir / "packages" / "web-browser.sh",
        install_dir / "packages" / "editors.sh",
        install_dir / "greeter" / "sddm" / "sddm.sh",
        install_dir / "post-install" / "gnome-theme.sh",
    ]

    return_code = install_base_packages(install_dir, log_file, colors)
    if return_code != 0:
        stop_install_log(log_file, start_time, colors)
        return return_code

    for script in scripts:
        if not script.exists():
            log_line(log_file, f"Skipped missing: {script}")
            continue
        return_code = run_script(script, install_dir, log_file, colors)
        if return_code != 0:
            stop_install_log(log_file, start_time, colors)
            return return_code

    stop_install_log(log_file, start_time, colors)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
