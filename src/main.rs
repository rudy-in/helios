use std::{fs, io};
use clap::Parser;
use std::collections::HashMap;
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{enable_raw_mode, disable_raw_mode},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
    widgets::{Block, Borders, Paragraph},
    layout::{Layout, Constraint, Direction},
    style::{Style, Color},
    text::{Span, Line, Text},
};
use walkdir::WalkDir;
use colored::Colorize;

#[derive(Parser, Debug)]
#[command(version, about, long_about = "Decentralized Search engine")]
struct Args {
    #[arg(short, long)]
    path: String,
}

#[derive(PartialEq, Clone, Copy, Debug)]
enum Mode {
    Safe,
    Insert,
    Command,
}

fn main() -> io::Result<()> {
    let dir = Args::parse();

    // --- Index files ---
    let mut files: Vec<_> = WalkDir::new(&dir.path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.path().to_path_buf())
        .collect();
    files.sort();

    let mut index: HashMap<String, Vec<String>> = HashMap::new();
    for file_path in files {
        if file_path.is_file() {
            match fs::read_to_string(&file_path) {
                Ok(contents) => {
                    let file_name = file_path.display().to_string();
                    for word in contents.split_whitespace() {
                        let clean = word
                            .trim_matches(|c: char| !c.is_alphanumeric())
                            .to_lowercase();
                        if !clean.is_empty() {
                            index.entry(clean).or_default().push(file_name.clone());
                        }
                    }
                }
                Err(_) => eprintln!(
                    "{} {}",
                    "[INFO] Skipping non-text file:".yellow(),
                    file_path.display().to_string().red()
                ),
            }
        }
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut query = String::new();
    let mut results: Vec<String> = Vec::new();
    let mut mode = Mode::Insert;
    let mut scroll: u16 = 0;

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints(
                    [
                        Constraint::Min(1),
                        Constraint::Length(3),
                        Constraint::Length(1),
                    ]
                    .as_ref(),
                )
                .split(f.area());

            // --- Results ---
            let results_widget = if !results.is_empty() {
                let lines: Vec<Line> = results.iter()
                    .map(|res| render_highlighted(&query, res))
                    .collect();
                Paragraph::new(Text::from(lines))
                    .block(Block::default().title("Results").borders(Borders::ALL))
                    .scroll((scroll, 0))
            } else {
                Paragraph::new("No results")
                    .block(Block::default().title("Results").borders(Borders::ALL))
            };
            f.render_widget(results_widget, chunks[0]);

            // --- Input ---
            let input_style = match mode {
                Mode::Command => Style::default().fg(Color::Yellow),
                Mode::Safe => Style::default().fg(Color::Blue),
                Mode::Insert => Style::default().fg(Color::White),
            };
            let title = match mode {
                Mode::Insert => "Query",
                Mode::Command => "Command",
                Mode::Safe => "SAFE",
            };
            let input_widget = Paragraph::new(query.as_str())
                .style(input_style)
                .block(Block::default().title(title).borders(Borders::ALL));
            f.render_widget(input_widget, chunks[1]);

            // --- Status Bar ---
            let total = results.len();
            let current = (scroll + 1).min(total as u16);
            let status_text = format!("Mode: {:?} | Result: {}/{} | Scroll: {}", mode, current, total, scroll);
            let status_bar = Paragraph::new(status_text)
                .style(Style::default().fg(Color::Black).bg(Color::White));
            f.render_widget(status_bar, chunks[2]);
        })?;

        if event::poll(std::time::Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                match (mode, key.code) {
                    // --- Insert Mode ---
                    (Mode::Insert, KeyCode::Char(':')) => { mode = Mode::Command; query.clear(); },
                    (Mode::Insert, KeyCode::Char(c)) => query.push(c),
                    (Mode::Insert, KeyCode::Backspace) => { let _ = query.pop(); },
                    (Mode::Insert, KeyCode::Enter) => {
                        if !query.is_empty() {
                            let q = query.to_lowercase();
                            if let Some(files) = index.get(&q) {
                                results = files.iter().map(|f| format!("'{}' found in {}", q, f)).collect();
                            } else {
                                results = vec![format!("'{}' not found in any file", q)];
                            }
                            scroll = 0;
                        }
                    },

                    // --- Command Mode ---
                    (Mode::Command, KeyCode::Char('q')) => break,
                    (Mode::Command, KeyCode::Char('i')) => { mode = Mode::Insert; query.clear(); },
                    (Mode::Command, KeyCode::Char('s')) => { mode = Mode::Safe; },

                    // --- Safe Mode ---
                    (Mode::Safe, KeyCode::Char('i')) => { mode = Mode::Insert; query.clear(); },
                    (Mode::Safe, KeyCode::Char('c')) => { mode = Mode::Command; },
                    (Mode::Safe, KeyCode::Char('q')) => break,

                    // --- Scroll ---
                    (Mode::Command, KeyCode::Down) | (Mode::Command, KeyCode::Char('j')) |
                    (Mode::Safe, KeyCode::Down) | (Mode::Safe, KeyCode::Char('j')) => {
                        if scroll < results.len().saturating_sub(1) as u16 { scroll += 1; }
                    },
                    (Mode::Command, KeyCode::Up) | (Mode::Command, KeyCode::Char('k')) |
                    (Mode::Safe, KeyCode::Up) | (Mode::Safe, KeyCode::Char('k')) => {
                        scroll = scroll.saturating_sub(1);
                    },

                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    crossterm::execute!(terminal.backend_mut(), crossterm::terminal::LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

fn render_highlighted<'a>(query: &str, content: &str) -> Line<'a> {
    let mut spans: Vec<Span> = Vec::new();
    let q = query.to_lowercase();
    for word in content.split_whitespace() {
        if word.to_lowercase().contains(&q) {
            spans.push(Span::styled(format!("{} ", word), Style::default().fg(Color::Black).bg(Color::Cyan)));
        } else {
            spans.push(Span::raw(format!("{} ", word)));
        }
    }
    Line::from(spans)
}
