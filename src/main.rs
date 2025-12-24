mod installer;
mod model;
mod packages;
mod selection;
mod ui;

use std::collections::VecDeque;
use std::io;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
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
use crate::packages::{load_packages, parse_packages_arg};
use crate::ui::{draw_ui, run_browser_selector, SPINNER_LEN};

const LOG_CAPACITY: usize = 200;

fn main() -> Result<()> {
    let packages_path = parse_packages_arg()
        .or_else(|| std::env::var("PALAWAN_PACKAGES_FILE").ok());
    let packages = load_packages(packages_path.as_deref()).context("load package list")?;

    enable_raw_mode().context("enable raw mode")?;
    clear_screen()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))
        .context("init terminal")?;

    let browser_selection = match run_browser_selector(&mut terminal)? {
        Some(selection) => selection,
        None => {
            disable_raw_mode().context("disable raw mode")?;
            let _ = clear_screen();
            return Ok(());
        }
    };

    let (tx, rx) = crossbeam_channel::unbounded();
    let (sudo_tx, sudo_rx) = crossbeam_channel::bounded(1);

    let installer_tx = tx.clone();
    thread::spawn(move || {
        if let Err(err) = run_installer(installer_tx, sudo_rx, packages, browser_selection) {
            let _ = tx.send(InstallerEvent::Done(Some(err.to_string())));
        }
    });

    clear_screen()?;
    let mut app = App {
        steps: STEP_NAMES
            .iter()
            .map(|name| Step {
                name: (*name).to_string(),
                status: StepStatus::Pending,
                err: None,
            })
            .collect(),
        progress: 0.0,
        logs: VecDeque::from(vec!["Starting Palawan installer...".to_string()]),
        spinner_idx: 0,
        done: false,
        err: None,
    };

    terminal.clear().context("clear terminal")?;
    terminal.draw(|f| draw_ui(f.size(), f, &app))?;

    let mut last_tick = Instant::now();
    let mut sudo_keepalive: Option<Arc<AtomicBool>> = None;
    if sudo_available() {
        sudo_keepalive = Some(start_sudo_keepalive());
    }
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
