use std::collections::HashSet;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};

use crate::model::{InstallerEvent, StepStatus};
use crate::selection::PackageSelection;

pub const STEP_NAMES: [&str; 7] = [
    "Installing base packages",
    "Installing yay",
    "Installing web browsers",
    "Installing terminals",
    "Installing editors",
    "Installing nvm",
    "Finalizing",
];

const STEP_BASE: usize = 0;
const STEP_YAY: usize = 1;
const STEP_BROWSERS: usize = 2;
const STEP_TERMINALS: usize = 3;
const STEP_EDITORS: usize = 4;
const STEP_NVM: usize = 5;
const STEP_FINAL: usize = 6;
const STEP_COUNT: f64 = STEP_NAMES.len() as f64;

struct ProgressState {
    package_set: HashSet<String>,
    seen: HashSet<String>,
    total: usize,
    installed: usize,
    weight: f64,
    offset: f64,
}

pub fn run_installer(
    tx: crossbeam_channel::Sender<InstallerEvent>,
    sudo_rx: crossbeam_channel::Receiver<()>,
    packages: Vec<String>,
    browser_selection: PackageSelection,
    terminal_selection: PackageSelection,
    editor_selection: PackageSelection,
    should_install_nvm: bool,
) -> Result<()> {
    send_event(
        &tx,
        InstallerEvent::Step {
            index: STEP_BASE,
            status: StepStatus::Running,
            err: None,
        },
    );
    send_event(&tx, InstallerEvent::Log("Installing base packages...".to_string()));
    send_event(&tx, InstallerEvent::Log(format!("Packages: {}", packages.join(", "))));

    ensure_sudo_ready(&tx, &sudo_rx)?;
    let mut package_set = HashSet::new();
    for pkg in &packages {
        package_set.insert(pkg.to_string());
    }
    let state = Arc::new(Mutex::new(ProgressState {
        package_set,
        seen: HashSet::new(),
        total: packages.len(),
        installed: 0,
        weight: 1.0 / STEP_COUNT,
        offset: 0.0,
    }));

    let args = build_pacman_args(&packages);
    if let Err(err) = run_command(&tx, "sudo", &args, None, Some(state)) {
        send_event(
            &tx,
            InstallerEvent::Step {
                index: STEP_BASE,
                status: StepStatus::Failed,
                err: Some(err.to_string()),
            },
        );
        return Err(err);
    }

    send_event(
        &tx,
        InstallerEvent::Step {
            index: STEP_BASE,
            status: StepStatus::Done,
            err: None,
        },
    );
    send_event(&tx, InstallerEvent::Progress(1.0 / STEP_COUNT));

    send_event(
        &tx,
        InstallerEvent::Step {
            index: STEP_YAY,
            status: StepStatus::Running,
            err: None,
        },
    );
    if let Err(err) = install_yay(&tx, &sudo_rx) {
        send_event(
            &tx,
            InstallerEvent::Step {
                index: STEP_YAY,
                status: StepStatus::Failed,
                err: Some(err.to_string()),
            },
        );
        return Err(err);
    }

    send_event(&tx, InstallerEvent::Progress(2.0 / STEP_COUNT));
    send_event(
        &tx,
        InstallerEvent::Step {
            index: STEP_YAY,
            status: StepStatus::Done,
            err: None,
        },
    );

    send_event(
        &tx,
        InstallerEvent::Step {
            index: STEP_BROWSERS,
            status: StepStatus::Running,
            err: None,
        },
    );
    let browsers_skipped = browser_selection.is_empty();
    if let Err(err) = install_browsers(&tx, &sudo_rx, &browser_selection) {
        send_event(
            &tx,
            InstallerEvent::Step {
                index: STEP_BROWSERS,
                status: StepStatus::Failed,
                err: Some(err.to_string()),
            },
        );
        return Err(err);
    }
    send_event(&tx, InstallerEvent::Progress(3.0 / STEP_COUNT));
    send_event(
        &tx,
        InstallerEvent::Step {
            index: STEP_BROWSERS,
            status: if browsers_skipped {
                StepStatus::Skipped
            } else {
                StepStatus::Done
            },
            err: None,
        },
    );

    send_event(
        &tx,
        InstallerEvent::Step {
            index: STEP_TERMINALS,
            status: StepStatus::Running,
            err: None,
        },
    );
    let terminals_skipped = terminal_selection.is_empty();
    if let Err(err) = install_terminals(&tx, &sudo_rx, &terminal_selection) {
        send_event(
            &tx,
            InstallerEvent::Step {
                index: STEP_TERMINALS,
                status: StepStatus::Failed,
                err: Some(err.to_string()),
            },
        );
        return Err(err);
    }
    send_event(&tx, InstallerEvent::Progress(4.0 / STEP_COUNT));
    send_event(
        &tx,
        InstallerEvent::Step {
            index: STEP_TERMINALS,
            status: if terminals_skipped {
                StepStatus::Skipped
            } else {
                StepStatus::Done
            },
            err: None,
        },
    );

    send_event(
        &tx,
        InstallerEvent::Step {
            index: STEP_EDITORS,
            status: StepStatus::Running,
            err: None,
        },
    );
    let editors_skipped = editor_selection.is_empty();
    if let Err(err) = install_editors(&tx, &sudo_rx, &editor_selection) {
        send_event(
            &tx,
            InstallerEvent::Step {
                index: STEP_EDITORS,
                status: StepStatus::Failed,
                err: Some(err.to_string()),
            },
        );
        return Err(err);
    }
    send_event(&tx, InstallerEvent::Progress(5.0 / STEP_COUNT));
    send_event(
        &tx,
        InstallerEvent::Step {
            index: STEP_EDITORS,
            status: if editors_skipped {
                StepStatus::Skipped
            } else {
                StepStatus::Done
            },
            err: None,
        },
    );

    send_event(
        &tx,
        InstallerEvent::Step {
            index: STEP_NVM,
            status: StepStatus::Running,
            err: None,
        },
    );
    if let Err(err) = install_nvm(&tx, &sudo_rx, should_install_nvm) {
        send_event(
            &tx,
            InstallerEvent::Step {
                index: STEP_NVM,
                status: StepStatus::Failed,
                err: Some(err.to_string()),
            },
        );
        return Err(err);
    }
    send_event(&tx, InstallerEvent::Progress(6.0 / STEP_COUNT));
    send_event(
        &tx,
        InstallerEvent::Step {
            index: STEP_NVM,
            status: if should_install_nvm {
                StepStatus::Done
            } else {
                StepStatus::Skipped
            },
            err: None,
        },
    );

    send_event(
        &tx,
        InstallerEvent::Step {
            index: STEP_FINAL,
            status: StepStatus::Running,
            err: None,
        },
    );
    send_event(&tx, InstallerEvent::Log("Finalizing...".to_string()));
    thread::sleep(Duration::from_millis(300));
    send_event(
        &tx,
        InstallerEvent::Step {
            index: STEP_FINAL,
            status: StepStatus::Done,
            err: None,
        },
    );
    send_event(&tx, InstallerEvent::Progress(1.0));
    send_event(&tx, InstallerEvent::Done(None));

    Ok(())
}

fn install_yay(
    tx: &crossbeam_channel::Sender<InstallerEvent>,
    sudo_rx: &crossbeam_channel::Receiver<()>,
) -> Result<()> {
    send_event(&tx, InstallerEvent::Log("Installing yay (AUR helper)...".to_string()));

    if std::env::var("USER").unwrap_or_default() == "root" || nix_euid_is_root() {
        anyhow::bail!("yay install must run as a non-root user");
    }

    ensure_sudo_ready(tx, sudo_rx)?;
    let deps = vec!["git".to_string(), "base-devel".to_string()];
    let args = build_pacman_args(&deps);
    run_command(tx, "sudo", &args, None, None)?;

    let yay_installed = Command::new("yay")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false);
    if yay_installed {
        send_event(&tx, InstallerEvent::Log("yay is already installed, skipping.".to_string()));
        return Ok(());
    }

    send_event(&tx, InstallerEvent::Log("yay not found, installing...".to_string()));
    let temp_dir = "/tmp/yay-bin";
    if Path::new(temp_dir).exists() {
        let _ = fs::remove_dir_all(temp_dir);
    }

    let clone_args = vec![
        "clone".to_string(),
        "https://aur.archlinux.org/yay-bin.git".to_string(),
        temp_dir.to_string(),
    ];
    run_command(tx, "git", &clone_args, None, None)?;

    ensure_sudo_ready(tx, sudo_rx)?;
    let makepkg_args = vec![
        "-si".to_string(),
        "--noconfirm".to_string(),
        "--needed".to_string(),
        "--syncdeps".to_string(),
    ];
    run_command(tx, "makepkg", &makepkg_args, Some(temp_dir), None)?;

    let _ = fs::remove_dir_all(temp_dir);
    Ok(())
}

fn install_browsers(
    tx: &crossbeam_channel::Sender<InstallerEvent>,
    sudo_rx: &crossbeam_channel::Receiver<()>,
    selection: &PackageSelection,
) -> Result<()> {
    if selection.is_empty() {
        send_event(&tx, InstallerEvent::Log("Skipping browser install.".to_string()));
        return Ok(());
    }

    send_event(&tx, InstallerEvent::Log("Installing web browsers...".to_string()));
    ensure_sudo_ready(tx, sudo_rx)?;

    let total = selection.pacman.len() + selection.yay.len();
    let state = Arc::new(Mutex::new(ProgressState {
        package_set: selection
            .pacman
            .iter()
            .chain(selection.yay.iter())
            .cloned()
            .collect(),
        seen: HashSet::new(),
        total,
        installed: 0,
        weight: 1.0 / STEP_COUNT,
        offset: 2.0 / STEP_COUNT,
    }));

    if !selection.pacman.is_empty() {
        let args = build_pacman_args(&selection.pacman);
        run_command(tx, "sudo", &args, None, Some(state.clone()))?;
    }

    if !selection.yay.is_empty() {
        let args = build_yay_args(&selection.yay);
        run_command(tx, "yay", &args, None, Some(state))?;
    }

    Ok(())
}

fn install_terminals(
    tx: &crossbeam_channel::Sender<InstallerEvent>,
    sudo_rx: &crossbeam_channel::Receiver<()>,
    selection: &PackageSelection,
) -> Result<()> {
    if selection.is_empty() {
        send_event(&tx, InstallerEvent::Log("Skipping terminal install.".to_string()));
        return Ok(());
    }

    send_event(&tx, InstallerEvent::Log("Installing terminals...".to_string()));
    ensure_sudo_ready(tx, sudo_rx)?;

    let total = selection.pacman.len();
    let state = Arc::new(Mutex::new(ProgressState {
        package_set: selection.pacman.iter().cloned().collect(),
        seen: HashSet::new(),
        total,
        installed: 0,
        weight: 1.0 / STEP_COUNT,
        offset: 3.0 / STEP_COUNT,
    }));

    if !selection.pacman.is_empty() {
        let args = build_pacman_args(&selection.pacman);
        run_command(tx, "sudo", &args, None, Some(state))?;
    }

    Ok(())
}

fn install_editors(
    tx: &crossbeam_channel::Sender<InstallerEvent>,
    sudo_rx: &crossbeam_channel::Receiver<()>,
    selection: &PackageSelection,
) -> Result<()> {
    if selection.is_empty() {
        send_event(&tx, InstallerEvent::Log("Skipping editor install.".to_string()));
        return Ok(());
    }

    send_event(&tx, InstallerEvent::Log("Installing editors...".to_string()));
    ensure_sudo_ready(tx, sudo_rx)?;

    let total = selection.pacman.len() + selection.yay.len();
    let state = Arc::new(Mutex::new(ProgressState {
        package_set: selection
            .pacman
            .iter()
            .chain(selection.yay.iter())
            .cloned()
            .collect(),
        seen: HashSet::new(),
        total,
        installed: 0,
        weight: 1.0 / STEP_COUNT,
        offset: 4.0 / STEP_COUNT,
    }));

    if !selection.pacman.is_empty() {
        let args = build_pacman_args(&selection.pacman);
        run_command(tx, "sudo", &args, None, Some(state.clone()))?;
    }

    if !selection.yay.is_empty() {
        let args = build_yay_args(&selection.yay);
        run_command(tx, "yay", &args, None, Some(state))?;
    }

    Ok(())
}

fn install_nvm(
    tx: &crossbeam_channel::Sender<InstallerEvent>,
    sudo_rx: &crossbeam_channel::Receiver<()>,
    install: bool,
) -> Result<()> {
    if !install {
        send_event(&tx, InstallerEvent::Log("Skipping nvm install.".to_string()));
        return Ok(());
    }

    send_event(
        &tx,
        InstallerEvent::Log("Installing nvm (Node Version Manager)...".to_string()),
    );
    ensure_sudo_ready(tx, sudo_rx)?;

    let args = build_yay_args(&["nvm".to_string()]);
    run_command(tx, "yay", &args, None, None)?;

    configure_nvm_shell(tx)?;

    let shell_path = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
    let uses_zsh = shell_path.ends_with("zsh");
    let shell_cmd = if uses_zsh { "zsh" } else { "bash" };
    let rc_file = if uses_zsh { "~/.zshrc" } else { "~/.bashrc" };
    let shell_args = vec![
        "-lc".to_string(),
        format!(
            "source {} \
&& source /usr/share/nvm/init-nvm.sh \
&& nvm install --lts \
&& nvm use --lts \
&& corepack enable \
&& corepack prepare pnpm@latest --activate",
            rc_file
        ),
    ];
    run_command(tx, shell_cmd, &shell_args, None, None)?;

    send_event(
        &tx,
        InstallerEvent::Log("nvm and Node.js LTS installation complete.".to_string()),
    );
    Ok(())
}

fn configure_nvm_shell(tx: &crossbeam_channel::Sender<InstallerEvent>) -> Result<()> {
    let home = std::env::var("HOME").context("resolve HOME")?;
    let bashrc = Path::new(&home).join(".bashrc");
    let zshrc = Path::new(&home).join(".zshrc");
    let init_line = "source /usr/share/nvm/init-nvm.sh";
    let snippet = format!("\n# Load nvm\n{}\n", init_line);

    for rc_path in [bashrc, zshrc] {
        if !rc_path.exists() {
            send_event(
                tx,
                InstallerEvent::Log(format!(
                    "Creating {} for nvm setup.",
                    rc_path.display()
                )),
            );
            fs::File::create(&rc_path)
                .with_context(|| format!("create {}", rc_path.display()))?;
        }

        let contents = fs::read_to_string(&rc_path).unwrap_or_default();
        if !contents.contains(init_line) {
            send_event(
                tx,
                InstallerEvent::Log(format!(
                    "Adding nvm init to {}.",
                    rc_path.display()
                )),
            );
            let mut file = fs::OpenOptions::new()
                .append(true)
                .open(&rc_path)
                .with_context(|| format!("open {}", rc_path.display()))?;
            file.write_all(snippet.as_bytes())
                .with_context(|| format!("write {}", rc_path.display()))?;
        } else {
            send_event(
                tx,
                InstallerEvent::Log(format!(
                    "nvm init already present in {}.",
                    rc_path.display()
                )),
            );
        }
    }

    Ok(())
}

fn nix_euid_is_root() -> bool {
    #[cfg(target_family = "unix")]
    {
        unsafe { libc::geteuid() == 0 }
    }
    #[cfg(not(target_family = "unix"))]
    {
        false
    }
}

pub fn ensure_sudo() -> Result<()> {
    let status = Command::new("sudo")
        .arg("-v")
        .status()
        .context("sudo -v")?;
    if status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("sudo authentication failed"))
    }
}

fn ensure_sudo_ready(
    tx: &crossbeam_channel::Sender<InstallerEvent>,
    sudo_rx: &crossbeam_channel::Receiver<()>,
) -> Result<()> {
    if sudo_available() {
        return Ok(());
    }
    send_event(tx, InstallerEvent::NeedSudo);
    sudo_rx.recv().context("waiting for sudo")?;
    if sudo_available() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("sudo authentication failed"))
    }
}

pub fn sudo_available() -> bool {
    Command::new("sudo")
        .arg("-n")
        .arg("true")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub fn start_sudo_keepalive() -> Arc<std::sync::atomic::AtomicBool> {
    let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let stop_flag = Arc::clone(&stop);
    thread::spawn(move || {
        while !stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
            thread::sleep(Duration::from_secs(60));
            let _ = Command::new("sudo")
                .arg("-v")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
    });
    stop
}

fn build_pacman_args(packages: &[String]) -> Vec<String> {
    let mut args = vec![
        "pacman".to_string(),
        "-S".to_string(),
        "--noconfirm".to_string(),
        "--needed".to_string(),
        "--noprogressbar".to_string(),
    ];
    args.extend(packages.iter().cloned());
    args
}

fn build_yay_args(packages: &[String]) -> Vec<String> {
    let mut args = vec![
        "-S".to_string(),
        "--noconfirm".to_string(),
        "--needed".to_string(),
    ];
    args.extend(packages.iter().cloned());
    args
}

fn run_command(
    tx: &crossbeam_channel::Sender<InstallerEvent>,
    command: &str,
    args: &[String],
    cwd: Option<&str>,
    state: Option<Arc<Mutex<ProgressState>>>,
) -> Result<()> {
    let mut child = Command::new(command)
        .args(args)
        .current_dir(cwd.unwrap_or("."))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("start {}", command))?;

    let stdout = child.stdout.take().context("capture stdout")?;
    let stderr = child.stderr.take().context("capture stderr")?;

    let tx_stdout = tx.clone();
    let stdout_state = state.clone();
    let stdout_handle = thread::spawn(move || read_stream(stdout, &tx_stdout, stdout_state));

    let tx_stderr = tx.clone();
    let stderr_state = state.clone();
    let stderr_handle = thread::spawn(move || read_stream(stderr, &tx_stderr, stderr_state));

    let status = child.wait().context("wait for command")?;
    let _ = stdout_handle.join();
    let _ = stderr_handle.join();

    if status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("command failed with status: {}", status))
    }
}

fn read_stream<R: io::Read>(
    reader: R,
    tx: &crossbeam_channel::Sender<InstallerEvent>,
    state: Option<Arc<Mutex<ProgressState>>>,
) {
    let buf = io::BufReader::new(reader);
    for line in buf.lines().flatten() {
        if line.trim().is_empty() {
            continue;
        }
        if let Some(shared) = &state {
            if let Some(pkg) = parse_pacman_install(&line) {
                let mut guard = shared.lock().unwrap();
                if guard.package_set.contains(&pkg) && guard.seen.insert(pkg) {
                    guard.installed += 1;
                    let progress =
                        guard.offset + (guard.installed as f64 / guard.total as f64) * guard.weight;
                    send_event(tx, InstallerEvent::Progress(progress));
                }
            }
        }
        let _ = tx.try_send(InstallerEvent::Log(line));
    }
}

fn parse_pacman_install(line: &str) -> Option<String> {
    let lower = line.trim().to_lowercase();
    let prefix = "installing ";
    if !lower.starts_with(prefix) {
        return None;
    }
    let rest = line.trim()[prefix.len()..].trim();
    let pkg = rest.split_whitespace().next()?;
    Some(pkg.trim_end_matches('.').to_string())
}

fn send_event(tx: &crossbeam_channel::Sender<InstallerEvent>, evt: InstallerEvent) {
    let _ = tx.try_send(evt);
}
