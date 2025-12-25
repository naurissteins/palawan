mod drivers;
mod installer;
mod model;
mod packages;
mod selection;
mod ui;

use std::collections::VecDeque;
use std::io;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, ClearType};
use crossterm::{cursor, execute, terminal::Clear};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::installer::{ensure_sudo, run_installer, start_sudo_keepalive, sudo_available, STEP_NAMES};
use crate::model::{App, InstallerEvent, Step, StepStatus};
use crate::packages::{load_base_packages, load_hyprland_packages, parse_packages_arg};
use crate::selection::{
    flags_for_npm_selection,
    flags_for_selection,
    labels_for_npm_selection,
    labels_for_selection,
    NpmSelection,
    PackageSelection,
    BROWSER_CHOICES,
    CODING_AGENT_CHOICES,
    EDITOR_CHOICES,
    TERMINAL_CHOICES,
};
use crate::ui::{
    draw_ui,
    run_browser_selector,
    run_coding_agent_selector,
    run_editor_selector,
    run_nvm_selector,
    run_nvidia_selector,
    run_review,
    run_terminal_selector,
    ReviewAction,
    ReviewItem,
    SPINNER_LEN,
};
use crate::drivers::{
    detect_gpu_vendors, detect_installed_nvidia_variant, driver_packages, format_gpu_summary,
    nvidia_driver_installed, GpuVendor,
};

const LOG_CAPACITY: usize = 200;

fn main() -> Result<()> {
    let packages_path = parse_packages_arg()
        .or_else(|| std::env::var("PALAWAN_PACKAGES_FILE").ok());
    let mut packages =
        load_base_packages(packages_path.as_deref()).context("load base package list")?;
    let hyprland_packages =
        load_hyprland_packages().context("load hyprland package list")?;

    enable_raw_mode().context("enable raw mode")?;
    clear_screen()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))
        .context("init terminal")?;

    let gpu_vendors = detect_gpu_override().unwrap_or_else(|| detect_gpu_vendors().unwrap_or_default());
    let installed_nvidia_variant = if gpu_vendors.contains(&GpuVendor::Nvidia) {
        detect_installed_nvidia_variant()
    } else {
        None
    };
    let chosen_nvidia_variant =
        if gpu_vendors.contains(&GpuVendor::Nvidia) && !nvidia_driver_installed() {
        match run_nvidia_selector(&mut terminal)? {
            Some(variant) => Some(variant),
            None => {
                disable_raw_mode().context("disable raw mode")?;
                let _ = clear_screen();
                return Ok(());
            }
        }
    } else {
        None
    };
    let nvidia_variant = chosen_nvidia_variant.or(installed_nvidia_variant);
    let driver_packages = driver_packages(&gpu_vendors, nvidia_variant);
    extend_unique(&mut packages, &driver_packages);

    let format_labels = |labels: Vec<String>| {
        if labels.is_empty() {
            "None".to_string()
        } else {
            labels.join(", ")
        }
    };

    let mut last_browser_selection = PackageSelection::default();
    let mut last_terminal_selection = PackageSelection::default();
    let mut last_editor_selection = PackageSelection::default();
    let mut last_install_nvm = false;
    let mut last_coding_agent_selection = NpmSelection::default();
    let mut has_previous = false;

    let (browser_selection, terminal_selection, editor_selection, install_nvm, coding_agent_selection) =
        loop {
            let browser_flags = if has_previous {
                Some(flags_for_selection(&last_browser_selection, &BROWSER_CHOICES))
            } else {
                None
            };
            let browser_selection = match run_browser_selector(
                &mut terminal,
                browser_flags.as_deref(),
            )? {
                Some(selection) => selection,
                None => {
                    disable_raw_mode().context("disable raw mode")?;
                    let _ = clear_screen();
                    return Ok(());
                }
            };
            let terminal_flags = if has_previous {
                Some(flags_for_selection(&last_terminal_selection, &TERMINAL_CHOICES))
            } else {
                None
            };
            let terminal_selection = match run_terminal_selector(
                &mut terminal,
                terminal_flags.as_deref(),
            )? {
                Some(selection) => selection,
                None => {
                    disable_raw_mode().context("disable raw mode")?;
                    let _ = clear_screen();
                    return Ok(());
                }
            };
            let editor_flags = if has_previous {
                Some(flags_for_selection(&last_editor_selection, &EDITOR_CHOICES))
            } else {
                None
            };
            let editor_selection =
                match run_editor_selector(&mut terminal, editor_flags.as_deref())? {
                    Some(selection) => selection,
                    None => {
                        disable_raw_mode().context("disable raw mode")?;
                        let _ = clear_screen();
                        return Ok(());
                    }
                };
            let install_nvm = match run_nvm_selector(
                &mut terminal,
                has_previous.then_some(last_install_nvm),
            )? {
                Some(selection) => selection,
                None => {
                    disable_raw_mode().context("disable raw mode")?;
                    let _ = clear_screen();
                    return Ok(());
                }
            };
            let coding_flags = if has_previous {
                Some(flags_for_npm_selection(
                    &last_coding_agent_selection,
                    &CODING_AGENT_CHOICES,
                ))
            } else {
                None
            };
            let coding_agent_selection = match run_coding_agent_selector(
                &mut terminal,
                coding_flags.as_deref(),
            )? {
                Some(selection) => selection,
                None => {
                    disable_raw_mode().context("disable raw mode")?;
                    let _ = clear_screen();
                    return Ok(());
                }
            };

            last_browser_selection = browser_selection.clone();
            last_terminal_selection = terminal_selection.clone();
            last_editor_selection = editor_selection.clone();
            last_install_nvm = install_nvm;
            last_coding_agent_selection = coding_agent_selection.clone();
            has_previous = true;

            let review_items = vec![
                ReviewItem {
                    label: "Web browsers".to_string(),
                    value: format_labels(labels_for_selection(
                        &browser_selection,
                        &BROWSER_CHOICES,
                    )),
                },
                ReviewItem {
                    label: "Terminals".to_string(),
                    value: format_labels(labels_for_selection(
                        &terminal_selection,
                        &TERMINAL_CHOICES,
                    )),
                },
                ReviewItem {
                    label: "Editors".to_string(),
                    value: format_labels(labels_for_selection(
                        &editor_selection,
                        &EDITOR_CHOICES,
                    )),
                },
                ReviewItem {
                    label: "Install nvm".to_string(),
                    value: if install_nvm {
                        "Yes".to_string()
                    } else {
                        "No".to_string()
                    },
                },
                ReviewItem {
                    label: "Coding agents".to_string(),
                    value: format_labels(labels_for_npm_selection(
                        &coding_agent_selection,
                        &CODING_AGENT_CHOICES,
                    )),
                },
            ];

            match run_review(&mut terminal, &review_items)? {
                ReviewAction::Confirm => {
                    break (
                        browser_selection,
                        terminal_selection,
                        editor_selection,
                        install_nvm,
                        coding_agent_selection,
                    )
                }
                ReviewAction::Edit => continue,
                ReviewAction::Quit => {
                    disable_raw_mode().context("disable raw mode")?;
                    let _ = clear_screen();
                    return Ok(());
                }
            }
        };

    let mut sudo_verified = sudo_available();
    let mut sudo_keepalive = if sudo_verified {
        Some(start_sudo_keepalive())
    } else {
        disable_raw_mode().context("disable raw mode")?;
        clear_screen()?;
        println!("Sudo password is required to continue.");
        ensure_sudo()?;
        sudo_verified = true;
        let keepalive = Some(start_sudo_keepalive());
        enable_raw_mode().context("enable raw mode")?;
        clear_screen()?;
        keepalive
    };

    let (tx, rx) = crossbeam_channel::unbounded();
    let (sudo_tx, sudo_rx) = crossbeam_channel::bounded(1);

    let browser_selection_for_install = browser_selection.clone();
    let terminal_selection_for_install = terminal_selection.clone();
    let editor_selection_for_install = editor_selection.clone();
    let coding_agent_selection_for_install = coding_agent_selection.clone();

    let installer_tx = tx.clone();
    thread::spawn(move || {
        if let Err(err) = run_installer(
            installer_tx,
            sudo_rx,
            packages,
            hyprland_packages,
            browser_selection_for_install,
            terminal_selection_for_install,
            editor_selection_for_install,
            install_nvm,
            coding_agent_selection_for_install,
        ) {
            let _ = tx.send(InstallerEvent::Done(Some(err.to_string())));
        }
    });

    clear_screen()?;
    let mut step_names: Vec<String> = STEP_NAMES.iter().map(|name| (*name).to_string()).collect();
    let browser_labels = labels_for_selection(&browser_selection, &BROWSER_CHOICES);
    if !browser_labels.is_empty() {
        step_names[3] = format!("Installing {}", browser_labels.join(", "));
    }
    let terminal_labels = labels_for_selection(&terminal_selection, &TERMINAL_CHOICES);
    if !terminal_labels.is_empty() {
        step_names[4] = format!("Installing {}", terminal_labels.join(", "));
    }
    let editor_labels = labels_for_selection(&editor_selection, &EDITOR_CHOICES);
    if !editor_labels.is_empty() {
        step_names[5] = format!("Installing {}", editor_labels.join(", "));
    }
    let coding_agent_labels = labels_for_npm_selection(&coding_agent_selection, &CODING_AGENT_CHOICES);
    if !coding_agent_labels.is_empty() {
        step_names[7] = format!("Installing {}", coding_agent_labels.join(", "));
    }

    let mut logs = VecDeque::from(vec!["Starting Palawan installer...".to_string()]);
    if sudo_verified {
        logs.push_back("Sudo verified.".to_string());
    }
    if let Some(summary) = format_gpu_summary(
        &gpu_vendors,
        nvidia_variant,
        installed_nvidia_variant,
    ) {
        logs.push_back(summary);
    }

    let mut app = App {
        steps: step_names
            .iter()
            .map(|name| Step {
                name: name.to_string(),
                status: StepStatus::Pending,
                err: None,
            })
            .collect(),
        progress: 0.0,
        logs,
        spinner_idx: 0,
        done: false,
        err: None,
    };

    terminal.clear().context("clear terminal")?;
    terminal.draw(|f| draw_ui(f.size(), f, &app))?;

    let mut last_tick = Instant::now();
    loop {
        terminal.draw(|f| draw_ui(f.size(), f, &app))?;

        let timeout = Duration::from_millis(100);
        if event::poll(timeout).context("poll events")? {
            if let Event::Key(key) = event::read().context("read event")? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        _ => {}
                    }
                }
            }
        }

        while let Ok(evt) = rx.try_recv() {
            match evt {
                InstallerEvent::NeedSudo => {
                    disable_raw_mode().context("disable raw mode")?;
                    clear_screen()?;
                    println!("Sudo password is required to continue.");
                    ensure_sudo()?;
                    if sudo_keepalive.is_none() {
                        sudo_keepalive = Some(start_sudo_keepalive());
                    }
                    enable_raw_mode().context("enable raw mode")?;
                    clear_screen()?;
                    let _ = sudo_tx.send(());
                }
                _ => handle_event(&mut app, evt),
            }
        }

        if last_tick.elapsed() >= Duration::from_millis(120) {
            app.spinner_idx = (app.spinner_idx + 1) % SPINNER_LEN;
            last_tick = Instant::now();
        }
    }

    disable_raw_mode().context("disable raw mode")?;
    let _ = clear_screen();
    if let Some(flag) = sudo_keepalive {
        flag.store(true, Ordering::Relaxed);
    }
    Ok(())
}

fn clear_screen() -> Result<()> {
    execute!(
        io::stdout(),
        Clear(ClearType::All),
        cursor::MoveTo(0, 0)
    )
    .context("clear screen")?;
    Ok(())
}

fn handle_event(app: &mut App, evt: InstallerEvent) {
    match evt {
        InstallerEvent::Log(line) => push_log(&mut app.logs, line),
        InstallerEvent::Progress(value) => app.progress = value,
        InstallerEvent::Step { index, status, err } => {
            if let Some(step) = app.steps.get_mut(index) {
                step.status = status;
                step.err = err;
            }
        }
        InstallerEvent::Done(err) => {
            app.done = true;
            app.err = err;
        }
        InstallerEvent::NeedSudo => {}
    }
}

fn push_log(logs: &mut VecDeque<String>, line: String) {
    if logs.len() >= LOG_CAPACITY {
        logs.pop_front();
    }
    logs.push_back(line);
}

fn extend_unique(target: &mut Vec<String>, values: &[String]) {
    for value in values {
        if !target.iter().any(|existing| existing == value) {
            target.push(value.clone());
        }
    }
}

fn detect_gpu_override() -> Option<std::collections::HashSet<GpuVendor>> {
    let value = std::env::var("PALAWAN_DEV_GPU").ok()?;
    let mut vendors = std::collections::HashSet::new();
    for token in value.split(',') {
        let normalized = token.trim().to_ascii_lowercase();
        match normalized.as_str() {
            "amd" => {
                vendors.insert(GpuVendor::Amd);
            }
            "intel" => {
                vendors.insert(GpuVendor::Intel);
            }
            "nvidia" => {
                vendors.insert(GpuVendor::Nvidia);
            }
            _ => {}
        }
    }
    if vendors.is_empty() {
        None
    } else {
        Some(vendors)
    }
}
