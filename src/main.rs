use std::collections::{HashSet, VecDeque};
use std::io::{self, BufRead};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, ClearType};
use crossterm::{execute, terminal::Clear};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, Paragraph, Wrap};
use ratatui::Terminal;

const LOG_CAPACITY: usize = 200;
const SPINNER: [&str; 4] = ["|", "/", "-", "\\"];
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
}

fn main() -> Result<()> {
    let packages = load_packages("packages/base.txt").context("load package list")?;
    let (tx, rx) = crossbeam_channel::unbounded();

    let installer_tx = tx.clone();
    thread::spawn(move || {
        if let Err(err) = run_installer(installer_tx, packages) {
            let _ = tx.send(InstallerEvent::Step {
                index: 0,
                status: StepStatus::Failed,
                err: Some(err.to_string()),
            });
            let _ = tx.send(InstallerEvent::Done(Some(err.to_string())));
        }
    });

    enable_raw_mode().context("enable raw mode")?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))
        .context("init terminal")?;
    execute!(io::stdout(), Clear(ClearType::All))?;

    let mut app = App {
        steps: vec![
            Step {
                name: "Installing base packages".to_string(),
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
            handle_event(&mut app, evt);
        }

        if last_tick.elapsed() >= Duration::from_millis(120) {
            app.spinner_idx = (app.spinner_idx + 1) % SPINNER.len();
            last_tick = Instant::now();
        }
    }

    disable_raw_mode().context("disable raw mode")?;
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
    }
}

fn push_log(logs: &mut VecDeque<String>, line: String) {
    if logs.len() >= LOG_CAPACITY {
        logs.pop_front();
    }
    logs.push_back(line);
}

fn run_installer(tx: crossbeam_channel::Sender<InstallerEvent>, packages: Vec<String>) -> Result<()> {
    send_event(&tx, InstallerEvent::Step { index: 0, status: StepStatus::Running, err: None });
    send_event(&tx, InstallerEvent::Log("Installing base packages...".to_string()));
    send_event(&tx, InstallerEvent::Log(format!("Packages: {}", packages.join(", "))));

    ensure_sudo()?;

    let mut package_set = HashSet::new();
    for pkg in &packages {
        package_set.insert(pkg.to_string());
    }
    let state = std::sync::Arc::new(std::sync::Mutex::new(ProgressState {
        package_set,
        seen: HashSet::new(),
        total: packages.len(),
        installed: 0,
    }));

    let args = build_pacman_args(&packages);
    run_command(&tx, "sudo", &args, Some(state))?;

    send_event(&tx, InstallerEvent::Step { index: 0, status: StepStatus::Done, err: None });
    send_event(&tx, InstallerEvent::Step { index: 1, status: StepStatus::Running, err: None });
    send_event(&tx, InstallerEvent::Log("Finalizing...".to_string()));
    thread::sleep(Duration::from_millis(300));
    send_event(&tx, InstallerEvent::Step { index: 1, status: StepStatus::Done, err: None });
    send_event(&tx, InstallerEvent::Progress(1.0));
    send_event(&tx, InstallerEvent::Done(None));

    Ok(())
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

fn run_command(
    tx: &crossbeam_channel::Sender<InstallerEvent>,
    command: &str,
    args: &[String],
    state: Option<std::sync::Arc<std::sync::Mutex<ProgressState>>>,
) -> Result<()> {
    let mut child = Command::new(command)
        .args(args)
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
                    let progress = guard.installed as f64 / guard.total as f64;
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

fn load_packages(path: &str) -> Result<Vec<String>> {
    let file = std::fs::File::open(path).with_context(|| format!("open {}", path))?;
    let reader = io::BufReader::new(file);
    let mut packages = Vec::new();
    for line in reader.lines().flatten() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        packages.push(trimmed.to_string());
    }
    if packages.is_empty() {
        anyhow::bail!("no packages found in {}", path);
    }
    Ok(packages)
}

fn send_event(tx: &crossbeam_channel::Sender<InstallerEvent>, evt: InstallerEvent) {
    let _ = tx.try_send(evt);
}
