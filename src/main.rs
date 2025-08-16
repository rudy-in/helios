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

#[derive(PartialEq, Clone, Copy)]
enum Mode {
    Insert,
    Command,
}

fn main() -> io::Result<()> {
    let dir = Args::parse();

    // collect all files recursively
    let mut files: Vec<_> = WalkDir::new(&dir.path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.path().to_path_buf())
        .collect();
    files.sort();

    // index words to file paths
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
                Err(_) => {
                    eprintln!(
                        "{} {}",
                        "[INFO] Skipping non-text file:".yellow(),
                        file_path.display().to_string().red()
                    );
                }
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
                .constraints([Constraint::Min(1), Constraint::Length(3)].as_ref())
                .split(f.area());

            // Highlight results if query exists
            if !query.is_empty() && !results.is_empty() {
                let mut lines: Vec<Line> = Vec::new();
                for result in &results {
                    lines.push(render_highlighted(&query, result));
                }
                let results_widget = Paragraph::new(Text::from(lines))
                    .block(Block::default().title("Results").borders(Borders::ALL))
                    .scroll((scroll, 0)); // apply scroll
                f.render_widget(results_widget, chunks[0]);
            } else {
                let empty = Paragraph::new("No results")
                    .block(Block::default().title("Results").borders(Borders::ALL));
                f.render_widget(empty, chunks[0]);
            }

            let input_style = if mode == Mode::Command {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::White)
            };

            let input_widget = Paragraph::new(query.as_str())
                .style(input_style)
                .block(
                    Block::default()
                        .title(if mode == Mode::Command { "Command" } else { "Query" })
                        .borders(Borders::ALL),
                );
            f.render_widget(input_widget, chunks[1]);
        })?;

        if event::poll(std::time::Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                match (mode, key.code) {
                    (Mode::Insert, KeyCode::Char(':')) => {
                        mode = Mode::Command;
                        query.clear();
                    }
                    (Mode::Insert, KeyCode::Char(c)) => query.push(c),
                    (Mode::Insert, KeyCode::Backspace) => {
                        query.pop();
                    }
                    (Mode::Insert, KeyCode::Enter) => {
                        if !query.is_empty() {
                            let q = query.clone().to_lowercase();
                            if let Some(files) = index.get(&q) {
                                results = files
                                    .iter()
                                    .map(|f| format!("'{}' found in {}", q, f))
                                    .collect();
                                scroll = 0; // reset scroll
                            } else {
                                results = vec![format!("'{}' not found in any file", q)];
                                scroll = 0;
                            }
                        }
                    }

                    (Mode::Command, KeyCode::Char('q')) => {
                        break; // quit
                    }
                    (Mode::Command, KeyCode::Char('i')) => {
                        mode = Mode::Insert; // back to insert
                        query.clear();
                    }

                    // scrolling
                    (_, KeyCode::Down) | (_, KeyCode::Char('j')) => {
                        scroll = scroll.saturating_add(1);
                    }
                    (_, KeyCode::Up) | (_, KeyCode::Char('k')) => {
                        scroll = scroll.saturating_sub(1);
                    }
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;

    Ok(())
}

fn render_highlighted<'a>(query: &str, content: &str) -> Line<'a> {
    let mut spans: Vec<Span> = Vec::new();

    for word in content.split_whitespace() {
        if word.to_lowercase().contains(&query.to_lowercase()) {
            spans.push(Span::styled(
                format!("{} ", word),
                Style::default().fg(Color::Black).bg(Color::Cyan),
            ));
        } else {
            spans.push(Span::raw(format!("{} ", word)));
        }
    }

    Line::from(spans)
}
