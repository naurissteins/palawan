use std::io;
use std::time::Duration;

use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::{Frame, Terminal};
use ratatui::backend::CrosstermBackend;

use crate::drivers::NvidiaVariant;
use crate::model::{App, Step, StepStatus};
use crate::selection::{
    selection_from_flags,
    selection_from_flags_for,
    BROWSER_CHOICES,
    PackageSelection,
    TERMINAL_CHOICES,
};

pub const SPINNER_LEN: usize = 4;
const SPINNER: [&str; SPINNER_LEN] = ["|", "/", "-", "\\"];
const PALAWAN_ART: [&str; 7] = [
    "                 ▄▄▄",
    "██████╗  █████╗ ██╗      █████╗ ██╗    ██╗ █████╗ ███╗   ██╗",
    "██╔══██╗██╔══██╗██║     ██╔══██╗██║    ██║██╔══██╗████╗  ██║",
    "██████╔╝███████║██║     ███████║██║ █╗ ██║███████║██╔██╗ ██║",
    "██╔═══╝ ██╔══██║██║     ██╔══██║██║███╗██║██╔══██║██║╚██╗██║",
    "██║     ██║  ██║███████╗██║  ██║╚███╔███╔╝██║  ██║██║ ╚████║",
    "╚═╝     ╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝ ╚══╝╚══╝ ╚═╝  ╚═╝╚═╝  ╚═══╝",
];

pub fn draw_ui(area: Rect, f: &mut Frame<'_>, app: &App) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(PALAWAN_ART.len() as u16),
            Constraint::Length(1),
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

    let title = Line::from(vec![Span::styled(
        "Palawan Installer",
        Style::default().fg(Color::LightRed).add_modifier(Modifier::BOLD),
    )]);
    let title_block = Paragraph::new(title).block(Block::default());
    f.render_widget(title_block, layout[1]);

    let progress = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title("Progress"))
        .gauge_style(Style::default().fg(Color::Cyan))
        .ratio(app.progress);
    f.render_widget(progress, layout[3]);

    let step_lines: Vec<Line> = app
        .steps
        .iter()
        .map(|step| render_step(step, app.spinner_idx))
        .collect();
    let steps = Paragraph::new(step_lines)
        .block(Block::default().borders(Borders::ALL).title("Steps"))
        .wrap(Wrap { trim: false });
    f.render_widget(steps, layout[4]);

    let log_lines: Vec<Line> = app
        .logs
        .iter()
        .map(|line| Line::from(Span::raw(line.clone())))
        .collect();
    let logs = Paragraph::new(log_lines)
        .block(Block::default().borders(Borders::ALL).title("Logs"))
        .wrap(Wrap { trim: false });
    f.render_widget(logs, layout[5]);

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
    f.render_widget(status_line, layout[6]);
}

fn draw_browser_selector(area: Rect, f: &mut Frame<'_>, cursor: usize, selected: &[bool]) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(PALAWAN_ART.len() as u16),
            Constraint::Length(1),
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
        Line::from(vec![
            Span::styled("Up/Down", Style::default().fg(Color::Yellow)),
            Span::raw(" to move, "),
            Span::styled("Space", Style::default().fg(Color::Yellow)),
            Span::raw(" to toggle, "),
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::raw(" to continue."),
        ]),
        Line::from("Press s to skip browser installs, q to quit."),
        Line::from("Tip: press Enter with none selected to skip."),
    ])
    .block(Block::default().borders(Borders::ALL).title("Controls"))
    .wrap(Wrap { trim: false });
    f.render_widget(help, layout[3]);

    let main_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(layout[4]);

    let selected_count = selected.iter().filter(|flag| **flag).count();
    let list_title = if selected_count == 0 {
        "Browsers (0 selected - Enter skips)".to_string()
    } else {
        format!("Browsers ({} selected)", selected_count)
    };
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
            let mut spans = vec![Span::raw(format!("{:>2}) ", idx + 1))];
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
    f.render_widget(footer, layout[5]);
}

fn render_step(step: &Step, spinner_idx: usize) -> Line<'static> {
    let icon = match step.status {
        StepStatus::Pending => "[ ]",
        StepStatus::Running => "[..]",
        StepStatus::Done => "[OK]",
        StepStatus::Skipped => "[SKIP]",
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
        StepStatus::Skipped => Style::default().fg(Color::LightYellow),
        StepStatus::Failed => Style::default().fg(Color::LightRed),
    }
}

pub fn run_browser_selector(
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

fn draw_terminal_selector(area: Rect, f: &mut Frame<'_>, cursor: usize, selected: &[bool]) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(PALAWAN_ART.len() as u16),
            Constraint::Length(1),
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
        "Choose Terminals",
        Style::default().fg(Color::LightRed).add_modifier(Modifier::BOLD),
    )]);
    let title_block = Paragraph::new(title).block(Block::default());
    f.render_widget(title_block, layout[1]);

    let help = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Up/Down", Style::default().fg(Color::Yellow)),
            Span::raw(" to move, "),
            Span::styled("Space", Style::default().fg(Color::Yellow)),
            Span::raw(" to toggle, "),
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::raw(" to continue."),
        ]),
        Line::from("Press s to skip terminal installs, q to quit."),
        Line::from("Tip: press Enter with none selected to skip."),
    ])
    .block(Block::default().borders(Borders::ALL).title("Controls"))
    .wrap(Wrap { trim: false });
    f.render_widget(help, layout[3]);

    let main_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(layout[4]);

    let selected_count = selected.iter().filter(|flag| **flag).count();
    let list_title = if selected_count == 0 {
        "Terminals (0 selected - Enter skips)".to_string()
    } else {
        format!("Terminals ({} selected)", selected_count)
    };
    let items: Vec<ListItem> = TERMINAL_CHOICES
        .iter()
        .enumerate()
        .map(|(idx, choice)| {
            let is_selected = selected.get(idx).copied().unwrap_or(false);
            let marker_span = if is_selected {
                Span::styled("[x]", Style::default().fg(Color::LightGreen))
            } else {
                Span::raw("[ ]")
            };
            let mut spans = vec![Span::raw(format!("{:>2}) ", idx + 1))];
            spans.push(marker_span);
            spans.push(Span::raw(" "));
            spans.push(Span::raw(choice.label));
            spans.push(Span::raw(" "));
            spans.push(Span::raw("("));
            spans.push(Span::styled("pacman", Style::default().fg(Color::Cyan)));
            spans.push(Span::raw(")"));
            ListItem::new(Line::from(spans))
        })
        .collect();
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(list_title))
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
    let mut state = ListState::default();
    if !TERMINAL_CHOICES.is_empty() {
        state.select(Some(cursor.min(TERMINAL_CHOICES.len() - 1)));
    }
    f.render_stateful_widget(list, main_layout[0], &mut state);

    let selected_items: Vec<ListItem> = TERMINAL_CHOICES
        .iter()
        .zip(selected.iter())
        .filter_map(|(choice, flag)| {
            if *flag {
                let spans = vec![
                    Span::styled("[x]", Style::default().fg(Color::LightGreen)),
                    Span::raw(" "),
                    Span::raw(choice.label),
                    Span::raw(" "),
                    Span::raw("("),
                    Span::styled("pacman", Style::default().fg(Color::Cyan)),
                    Span::raw(")"),
                ];
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
    f.render_widget(footer, layout[5]);
}

pub fn run_terminal_selector(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<Option<PackageSelection>> {
    if TERMINAL_CHOICES.is_empty() {
        return Ok(Some(PackageSelection::default()));
    }

    let mut cursor: usize = 0;
    let mut selected = vec![false; TERMINAL_CHOICES.len()];
    loop {
        terminal.draw(|f| draw_terminal_selector(f.size(), f, cursor, &selected))?;

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
                        if cursor + 1 < TERMINAL_CHOICES.len() {
                            cursor += 1;
                        }
                    }
                    KeyCode::Char(' ') => {
                        if let Some(flag) = selected.get_mut(cursor) {
                            *flag = !*flag;
                        }
                    }
                    KeyCode::Enter => {
                        let selection = selection_from_flags_for(&selected, &TERMINAL_CHOICES);
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
                            if idx > 0 && idx <= TERMINAL_CHOICES.len() {
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

pub fn run_nvidia_selector(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<Option<NvidiaVariant>> {
    let options = [
        ("Open kernel module (Turing+)", NvidiaVariant::Open),
        ("Proprietary driver", NvidiaVariant::Proprietary),
        ("Open-source nouveau", NvidiaVariant::Nouveau),
    ];
    let mut cursor: usize = 0;
    loop {
        terminal.draw(|f| draw_nvidia_selector(f.size(), f, cursor, &options))?;

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
                        if cursor + 1 < options.len() {
                            cursor += 1;
                        }
                    }
                    KeyCode::Enter => {
                        return Ok(Some(options[cursor].1));
                    }
                    KeyCode::Char('q') | KeyCode::Esc => {
                        return Ok(None);
                    }
                    _ => {}
                }
            }
        }
    }
}

fn draw_nvidia_selector(
    area: Rect,
    f: &mut Frame<'_>,
    cursor: usize,
    options: &[(&str, NvidiaVariant)],
) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(PALAWAN_ART.len() as u16),
            Constraint::Length(1),
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
        "Choose NVIDIA Driver",
        Style::default().fg(Color::LightRed).add_modifier(Modifier::BOLD),
    )]);
    let title_block = Paragraph::new(title).block(Block::default());
    f.render_widget(title_block, layout[1]);

    let help = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Up/Down", Style::default().fg(Color::Yellow)),
            Span::raw(" to move, "),
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::raw(" to select."),
        ]),
        Line::from("Press q to quit."),
    ])
    .block(Block::default().borders(Borders::ALL).title("Controls"))
    .wrap(Wrap { trim: false });
    f.render_widget(help, layout[3]);

    let items: Vec<ListItem> = options
        .iter()
        .enumerate()
        .map(|(idx, (label, _))| {
            ListItem::new(Line::from(format!("{:>2}) {}", idx + 1, label)))
        })
        .collect();
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("NVIDIA options"))
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
    let mut state = ListState::default();
    state.select(Some(cursor.min(options.len().saturating_sub(1))));
    f.render_stateful_widget(list, layout[4], &mut state);

    let footer = Paragraph::new(Line::from(Span::styled(
        "Choose the driver variant you prefer.",
        Style::default().fg(Color::Gray),
    )));
    f.render_widget(footer, layout[5]);
}
