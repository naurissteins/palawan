use std::collections::{HashSet, VecDeque};
use std::fs;
use std::io::{self, BufRead};
use std::path::Path;
use std::process::{Command, Stdio};
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
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Terminal;

const LOG_CAPACITY: usize = 200;
const SPINNER: [&str; 4] = ["|", "/", "-", "\\"];
const DEFAULT_PACKAGES: &str = include_str!("../packages/base.txt");
const STEP_BASE: usize = 0;
const STEP_YAY: usize = 1;
const STEP_BROWSERS: usize = 2;
const STEP_FINAL: usize = 3;
const STEP_COUNT: f64 = 4.0;
const FIREFOX_PACMAN: [&str; 2] = ["firefox", "firefox-ublock-origin"];
const CHROMIUM_PACMAN: [&str; 1] = ["chromium"];
const UNGOOGLED_YAY: [&str; 1] = ["ungoogled-chromium-bin"];
const BRAVE_YAY: [&str; 1] = ["brave-bin"];
const ZEN_YAY: [&str; 1] = ["zen-browser-bin"];
const LIBREWOLF_YAY: [&str; 1] = ["librewolf-bin"];
const PALAWAN_ART: [&str; 7] = [
    "                 ▄▄▄",
    "██████╗  █████╗ ██╗      █████╗ ██╗    ██╗ █████╗ ███╗   ██╗",
    "██╔══██╗██╔══██╗██║     ██╔══██╗██║    ██║██╔══██╗████╗  ██║",
    "██████╔╝███████║██║     ███████║██║ █╗ ██║███████║██╔██╗ ██║",
    "██╔═══╝ ██╔══██║██║     ██╔══██║██║███╗██║██╔══██║██║╚██╗██║",
    "██║     ██║  ██║███████╗██║  ██║╚███╔███╔╝██║  ██║██║ ╚████║",
    "╚═╝     ╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝ ╚══╝╚══╝ ╚═╝  ╚═╝╚═╝  ╚═══╝",
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum StepStatus {
    Pending,
    Running,
    Done,
    Failed,
}

struct Step {
    name: String,
    status: StepStatus,
    err: Option<String>,
}

enum InstallerEvent {
    Log(String),
    Progress(f64),
    Step { index: usize, status: StepStatus, err: Option<String> },
    Done(Option<String>),
    NeedSudo,
}

struct App {
    steps: Vec<Step>,
    progress: f64,
    logs: VecDeque<String>,
    spinner_idx: usize,
    done: bool,
    err: Option<String>,
}

struct ProgressState {
    package_set: HashSet<String>,
    seen: HashSet<String>,
    total: usize,
    installed: usize,
    weight: f64,
    offset: f64,
}

#[derive(Default, Clone)]
struct PackageSelection {
    pacman: Vec<String>,
    yay: Vec<String>,
}

impl PackageSelection {
    fn is_empty(&self) -> bool {
        self.pacman.is_empty() && self.yay.is_empty()
    }
}

struct InstallChoice {
    label: &'static str,
    pacman: &'static [&'static str],
    yay: &'static [&'static str],
}

const BROWSER_CHOICES: [InstallChoice; 6] = [
    InstallChoice {
        label: "Firefox",
        pacman: &FIREFOX_PACMAN,
        yay: &[],
    },
    InstallChoice {
        label: "Chromium",
        pacman: &CHROMIUM_PACMAN,
        yay: &[],
    },
    InstallChoice {
        label: "Ungoogled Chromium (AUR)",
        pacman: &[],
        yay: &UNGOOGLED_YAY,
    },
    InstallChoice {
        label: "Brave (AUR)",
        pacman: &[],
        yay: &BRAVE_YAY,
    },
    InstallChoice {
        label: "Zen Browser (AUR)",
        pacman: &[],
        yay: &ZEN_YAY,
    },
    InstallChoice {
        label: "LibreWolf (AUR)",
        pacman: &[],
        yay: &LIBREWOLF_YAY,
    },
];

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
        steps: vec![
            Step {
                name: "Installing base packages".to_string(),
                status: StepStatus::Pending,
                err: None,
            },
            Step {
                name: "Installing yay".to_string(),
                status: StepStatus::Pending,
                err: None,
            },
            Step {
                name: "Installing web browsers".to_string(),
                status: StepStatus::Pending,
                err: None,
            },
            Step {
                name: "Finalizing".to_string(),
                status: StepStatus::Pending,
                err: None,
            },
        ],
        progress: 0.0,
        logs: VecDeque::from(vec!["Starting Palawan installer...".to_string()]),
        spinner_idx: 0,
        done: false,
        err: None,
    };

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
            app.spinner_idx = (app.spinner_idx + 1) % SPINNER.len();
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

fn draw_ui(area: Rect, f: &mut ratatui::Frame<'_>, app: &App) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(PALAWAN_ART.len() as u16),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(app.steps.len() as u16 + 2),
            Constraint::Min(6),
            Constraint::Length(1),
        ])
        .split(area);

    let art_lines: Vec<Line> = PALAWAN_ART
        .iter()
        .map(|line| {
            Line::from(Span::styled(
                *line,
                Style::default()
                    .fg(Color::LightRed)
                    .add_modifier(Modifier::BOLD),
            ))
        })
        .collect();
    let art = Paragraph::new(art_lines).block(Block::default());
    f.render_widget(art, layout[0]);

    let title = Line::from(vec![
        Span::styled(
            "Palawan Installer",
            Style::default().fg(Color::LightRed).add_modifier(Modifier::BOLD),
        ),
    ]);
    let title_block = Paragraph::new(title).block(Block::default());
    f.render_widget(title_block, layout[1]);

    let progress = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title("Progress"))
        .gauge_style(Style::default().fg(Color::Cyan))
        .ratio(app.progress);
    f.render_widget(progress, layout[2]);

    let step_lines: Vec<Line> = app
        .steps
        .iter()
        .map(|step| render_step(step, app.spinner_idx))
        .collect();
    let steps = Paragraph::new(step_lines)
        .block(Block::default().borders(Borders::ALL).title("Steps"))
        .wrap(Wrap { trim: false });
    f.render_widget(steps, layout[3]);

    let log_lines: Vec<Line> = app
        .logs
        .iter()
        .map(|line| Line::from(Span::raw(line.clone())))
        .collect();
    let logs = Paragraph::new(log_lines)
        .block(Block::default().borders(Borders::ALL).title("Logs"))
        .wrap(Wrap { trim: false });
    f.render_widget(logs, layout[4]);

    let status = if app.done {
        if app.err.is_some() {
            "Installation failed. Press q to quit."
        } else {
            "Installation complete. Press q to quit."
        }
    } else {
        "Press q to quit."
    };
    let status_style = if app.err.is_some() {
        Style::default().fg(Color::LightRed)
    } else if app.done {
        Style::default().fg(Color::LightGreen)
    } else {
        Style::default().fg(Color::Gray)
    };
    let status_line = Paragraph::new(Line::from(Span::styled(status, status_style)));
    f.render_widget(status_line, layout[5]);
}

fn draw_browser_selector(
    area: Rect,
    f: &mut ratatui::Frame<'_>,
    cursor: usize,
    selected: &[bool],
) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(PALAWAN_ART.len() as u16),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Min(6),
            Constraint::Length(1),
        ])
        .split(area);

    let art_lines: Vec<Line> = PALAWAN_ART
        .iter()
        .map(|line| {
            Line::from(Span::styled(
                *line,
                Style::default()
                    .fg(Color::LightRed)
                    .add_modifier(Modifier::BOLD),
            ))
        })
        .collect();
    let art = Paragraph::new(art_lines).block(Block::default());
    f.render_widget(art, layout[0]);

    let title = Line::from(vec![Span::styled(
        "Choose Web Browsers",
        Style::default().fg(Color::LightRed).add_modifier(Modifier::BOLD),
    )]);
    let title_block = Paragraph::new(title).block(Block::default());
    f.render_widget(title_block, layout[1]);

    let help = Paragraph::new(vec![
        Line::from("Up/Down to move, Space to toggle, Enter to continue."),
        Line::from("Press s to skip browser installs, q to quit."),
    ])
    .block(Block::default().borders(Borders::ALL).title("Controls"))
    .wrap(Wrap { trim: false });
    f.render_widget(help, layout[2]);

    let main_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(layout[3]);

    let selected_count = selected.iter().filter(|flag| **flag).count();
    let list_title = format!("Browsers ({} selected)", selected_count);
    let items: Vec<ListItem> = BROWSER_CHOICES
        .iter()
        .enumerate()
        .map(|(idx, choice)| {
            let is_selected = selected.get(idx).copied().unwrap_or(false);
            let marker_span = if is_selected {
                Span::styled("[x]", Style::default().fg(Color::LightGreen))
            } else {
                Span::raw("[ ]")
            };
            let mut spans = vec![Span::raw(format!(
                "{:>2}) ",
                idx + 1,
            ))];
            spans.push(marker_span);
            spans.push(Span::raw(" "));
            spans.push(Span::raw(choice.label));
            spans.push(Span::raw(" "));
            let has_pacman = !choice.pacman.is_empty();
            let has_yay = !choice.yay.is_empty();
            spans.push(Span::raw("("));
            if has_pacman {
                spans.push(Span::styled("pacman", Style::default().fg(Color::Cyan)));
            }
            if has_pacman && has_yay {
                spans.push(Span::raw(" + "));
            }
            if has_yay {
                spans.push(Span::styled("AUR", Style::default().fg(Color::Yellow)));
            }
            spans.push(Span::raw(")"));
            ListItem::new(Line::from(spans))
        })
        .collect();
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(list_title))
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
    let mut state = ListState::default();
    if !BROWSER_CHOICES.is_empty() {
        state.select(Some(cursor.min(BROWSER_CHOICES.len() - 1)));
    }
    f.render_stateful_widget(list, main_layout[0], &mut state);

    let selected_items: Vec<ListItem> = BROWSER_CHOICES
        .iter()
        .zip(selected.iter())
        .filter_map(|(choice, flag)| {
            if *flag {
                let mut spans = vec![
                    Span::styled("[x]", Style::default().fg(Color::LightGreen)),
                    Span::raw(" "),
                    Span::raw(choice.label),
                    Span::raw(" "),
                ];
                let has_pacman = !choice.pacman.is_empty();
                let has_yay = !choice.yay.is_empty();
                spans.push(Span::raw("("));
                if has_pacman {
                    spans.push(Span::styled("pacman", Style::default().fg(Color::Cyan)));
                }
                if has_pacman && has_yay {
                    spans.push(Span::raw(" + "));
                }
                if has_yay {
                    spans.push(Span::styled("AUR", Style::default().fg(Color::Yellow)));
                }
                spans.push(Span::raw(")"));
                Some(ListItem::new(Line::from(spans)))
            } else {
                None
            }
        })
        .collect();
    let selected_block = if selected_items.is_empty() {
        List::new(vec![ListItem::new(Line::from("None selected"))])
            .block(Block::default().borders(Borders::ALL).title("Selection"))
    } else {
        List::new(selected_items)
            .block(Block::default().borders(Borders::ALL).title("Selection"))
    };
    f.render_widget(selected_block, main_layout[1]);

    let footer = Paragraph::new(Line::from(Span::styled(
        "Selections apply to this run only.",
        Style::default().fg(Color::Gray),
    )));
    f.render_widget(footer, layout[4]);
}

fn render_step(step: &Step, spinner_idx: usize) -> Line<'static> {
    let icon = match step.status {
        StepStatus::Pending => "[ ]",
        StepStatus::Running => "[..]",
        StepStatus::Done => "[OK]",
        StepStatus::Failed => "[x]",
    };

    let mut spans = vec![Span::styled(
        format!("{} {}", icon, step.name),
        style_for_status(step.status),
    )];
    if step.status == StepStatus::Running {
        spans.push(Span::raw(format!(" {}", SPINNER[spinner_idx])));
    }
    if let Some(err) = &step.err {
        spans.push(Span::styled(
            format!(" ({})", err),
            Style::default().fg(Color::LightRed),
        ));
    }

    Line::from(spans)
}

fn style_for_status(status: StepStatus) -> Style {
    match status {
        StepStatus::Pending => Style::default().fg(Color::Gray),
        StepStatus::Running => Style::default().fg(Color::Yellow),
        StepStatus::Done => Style::default().fg(Color::LightGreen),
        StepStatus::Failed => Style::default().fg(Color::LightRed),
    }
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

fn run_installer(
    tx: crossbeam_channel::Sender<InstallerEvent>,
    sudo_rx: crossbeam_channel::Receiver<()>,
    packages: Vec<String>,
    browser_selection: PackageSelection,
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
    let state = std::sync::Arc::new(std::sync::Mutex::new(ProgressState {
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
            status: StepStatus::Done,
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
    let state = std::sync::Arc::new(std::sync::Mutex::new(ProgressState {
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

fn run_browser_selector(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<Option<PackageSelection>> {
    if BROWSER_CHOICES.is_empty() {
        return Ok(Some(PackageSelection::default()));
    }

    let mut cursor: usize = 0;
    let mut selected = vec![false; BROWSER_CHOICES.len()];
    loop {
        terminal.draw(|f| draw_browser_selector(f.size(), f, cursor, &selected))?;

        let timeout = Duration::from_millis(100);
        if event::poll(timeout).context("poll events")? {
            if let Event::Key(key) = event::read().context("read event")? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Up => {
                        if cursor > 0 {
                            cursor -= 1;
                        }
                    }
                    KeyCode::Down => {
                        if cursor + 1 < BROWSER_CHOICES.len() {
                            cursor += 1;
                        }
                    }
                    KeyCode::Char(' ') => {
                        if let Some(flag) = selected.get_mut(cursor) {
                            *flag = !*flag;
                        }
                    }
                    KeyCode::Enter => {
                        let selection = selection_from_flags(&selected);
                        return Ok(Some(selection));
                    }
                    KeyCode::Char('s') => {
                        return Ok(Some(PackageSelection::default()));
                    }
                    KeyCode::Char('q') | KeyCode::Esc => {
                        return Ok(None);
                    }
                    KeyCode::Char(ch) if ch.is_ascii_digit() => {
                        if let Some(index) = ch.to_digit(10) {
                            let idx = index as usize;
                            if idx > 0 && idx <= BROWSER_CHOICES.len() {
                                let pos = idx - 1;
                                selected[pos] = !selected[pos];
                                cursor = pos;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

    }
}

fn selection_from_flags(flags: &[bool]) -> PackageSelection {
    let mut selection = PackageSelection::default();
    for (flag, choice) in flags.iter().copied().zip(BROWSER_CHOICES.iter()) {
        if flag {
            extend_unique(&mut selection.pacman, choice.pacman);
            extend_unique(&mut selection.yay, choice.yay);
        }
    }
    selection
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

fn ensure_sudo() -> Result<()> {
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

fn sudo_available() -> bool {
    Command::new("sudo")
        .arg("-n")
        .arg("true")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn start_sudo_keepalive() -> Arc<AtomicBool> {
    let stop = Arc::new(AtomicBool::new(false));
    let stop_flag = Arc::clone(&stop);
    thread::spawn(move || {
        while !stop_flag.load(Ordering::Relaxed) {
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
    state: Option<std::sync::Arc<std::sync::Mutex<ProgressState>>>,
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
    state: Option<std::sync::Arc<std::sync::Mutex<ProgressState>>>,
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

fn load_packages(path: Option<&str>) -> Result<Vec<String>> {
    match path {
        Some(path) => {
            let file = std::fs::File::open(path).with_context(|| format!("open {}", path))?;
            let reader = io::BufReader::new(file);
            parse_packages(reader, Some(path))
        }
        None => parse_packages(DEFAULT_PACKAGES.as_bytes(), None),
    }
}

fn parse_packages<R: io::Read>(reader: R, source: Option<&str>) -> Result<Vec<String>> {
    let buf = io::BufReader::new(reader);
    let mut packages = Vec::new();
    for line in buf.lines().flatten() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        packages.push(trimmed.to_string());
    }
    if packages.is_empty() {
        let source = source.unwrap_or("embedded package list");
        anyhow::bail!("no packages found in {}", source);
    }
    Ok(packages)
}

fn send_event(tx: &crossbeam_channel::Sender<InstallerEvent>, evt: InstallerEvent) {
    let _ = tx.try_send(evt);
}

fn parse_packages_arg() -> Option<String> {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--packages-file" {
            return args.next();
        }
        if let Some(value) = arg.strip_prefix("--packages-file=") {
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

fn extend_unique(target: &mut Vec<String>, values: &[&str]) {
    for value in values {
        if !target.iter().any(|existing| existing == value) {
            target.push(value.to_string());
        }
    }
}
