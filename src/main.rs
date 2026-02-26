use std::{
    env, 
    fs, 
    io,
    time::{Duration, Instant},
};

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph},
    Terminal,
};

fn main() -> io::Result<()> {
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    let backend = CrosstermBackend::new(&mut stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    loop {
        let (wpm, acc) = run_typing_test(&mut terminal)?;
        let retry = ask_retry_tui(&mut terminal, wpm, acc)?;
        if !retry {
            break;
        }
    }

    crossterm::terminal::disable_raw_mode()?;
    terminal.clear()?;
    Ok(())
}

fn run_typing_test(
    terminal: &mut Terminal<CrosstermBackend<&mut io::Stdout>>,
) -> io::Result<(f64, f64)> {
    let args: Vec<String> = env::args().collect();
    let prompt = if args.len() > 1 {
        fs::read_to_string(&args[1]).unwrap_or_else(|_| "Failed to load file.".to_string())
    } else {
        "The quick brown fox jumps over the lazy dog.".to_string()
    };

    let total_time_secs: u64 = 60;
    let mut typed = String::new();
    let mut start_time = None::<Instant>;

    loop {
        // Time tracking
        let (_, remaining_secs) = if let Some(start) = start_time {
            let elapsed = start.elapsed().as_secs();
            let remain = total_time_secs.saturating_sub(elapsed);
            (elapsed, remain)
        } else {
            (0, total_time_secs)
        };

        if remaining_secs == 0 {
            break;
        }

        // Stats
        let (wpm, accuracy) = if let Some(start) = start_time {
            let elapsed_minutes = start.elapsed().as_secs_f64() / 60.0;
            let total_chars = typed.chars().count().max(1);
            let correct = prompt
                .chars()
                .zip(typed.chars())
                .filter(|(a, b)| a == b)
                .count();
            let acc = correct as f64 / total_chars as f64 * 100.0;
            let wpm = if elapsed_minutes > 0.0 {
                (typed.chars().count() as f64 / 5.0) / elapsed_minutes
            } else {
                0.0
            };
            (wpm, acc)
        } else {
            (0.0, 100.0)
        };

        // Text color change for accuracy
        let mut spans = Vec::new();
        let typed_len = typed.chars().count();
        for (i, c) in prompt.chars().enumerate() {
            let style = if i < typed_len {
                if typed.chars().nth(i) == Some(c) {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::Red)
                }
            } else {
                Style::default().fg(Color::DarkGray)
            };
            spans.push(Span::styled(c.to_string(), style));
        }

        // Progress
        let progress = (typed_len as f64 / prompt.chars().count() as f64) 
            .clamp(0.0, 1.0);

        // Layout
        terminal.draw(|f| {
            let layout = Layout::default()
                .direction(Direction::Vertical)
                .margin(2)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Min(1),
                ])
                .split(f.area());

            let prompt_para = Paragraph::new(Line::from(spans))
                .block(Block::default().borders(Borders::ALL).title("Prompt"));

            let typed_para = Paragraph::new(typed.clone())
                .block(Block::default().borders(Borders::ALL).title("Your Input"));

            let stats = Paragraph::new(Line::from(vec![
                Span::styled(
                    format!("WPM: {:>5.1}", wpm),
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                ),
                Span::raw("    "),
                Span::styled(
                    format!("Accuracy: {:>5.1}%", accuracy),
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                ),
            ]))
            .block(Block::default().borders(Borders::ALL).title("Stats"));

            let gauge = Gauge::default()
                .block(Block::default().borders(Borders::ALL).title("Progress"))
                .gauge_style(
                    Style::default()
                        .fg(Color::Green)
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD),
                )
                .ratio(progress)
                .label(format!("{:.0}%", progress * 100.0))
                .use_unicode(true);

            let timer = Paragraph::new(Line::from(vec![Span::styled(
                format!("Time Left: {:02}s", remaining_secs),
                Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
            )]))
            .block(Block::default().borders(Borders::ALL).title("Timer"));

            f.render_widget(prompt_para, layout[0]);
            f.render_widget(typed_para, layout[1]);
            f.render_widget(stats, layout[2]);
            f.render_widget(gauge, layout[3]);
            f.render_widget(timer, layout[4]);
        })?;

        // Input handling
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                // Adjustment made here for Windows input errors when typing; double typing on single key press
                if key.kind == KeyEventKind::Press{
                    match key.code {
                        KeyCode::Char(c) => {
                            if start_time.is_none() {
                                start_time = Some(Instant::now());
                            }
                            typed.push(c);
                        }
                        KeyCode::Backspace => {
                            typed.pop();
                        }
                        KeyCode::Esc | KeyCode::Enter => break,
                        _ => {}
                    }
                }
            }
        }
    }
    // Results
    let (final_wpm, final_accuracy) = if let Some(start) = start_time {
        let elapsed_minutes = start.elapsed().as_secs_f64() / 60.0;
        let total_chars = typed.chars().count().max(1);
        let correct = prompt
            .chars()
            .zip(typed.chars())
            .filter(|(a, b)| a == b)
            .count();
        let acc = correct as f64 / total_chars as f64 * 100.0;
        let wpm = if elapsed_minutes > 0.0 {
            (typed.chars().count() as f64 / 5.0) / elapsed_minutes
        } else {
            0.0
        };
        (wpm, acc)
    } else {
        (0.0, 100.0)
    };

    Ok((final_wpm, final_accuracy))
}

fn ask_retry_tui(
    terminal: &mut Terminal<CrosstermBackend<&mut io::Stdout>>,
    wpm: f64,
    acc: f64,
) -> io::Result<bool> {
    loop {
        terminal.draw(|f| {
            let area = f.area();
            let popup_area = centered_rect(40, 20, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .title("Results")
                .style(Style::default().fg(Color::Cyan));

            let text = vec![
                Line::from(Span::styled(
                    format!("WPM: {:.1}", wpm),
                    Style::default().fg(Color::Cyan),
                )),
                Line::from(Span::styled(
                    format!("Accuracy: {:.1}%", acc),
                    Style::default().fg(Color::Yellow),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Try again? [Y/N]",
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )),
            ];

            let para = Paragraph::new(text)
                .block(block)
                .alignment(Alignment::Center);

            f.render_widget(para, popup_area);
        })?;

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        terminal.clear()?;
                        return Ok(true);
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                        terminal.clear()?;
                        return Ok(false);
                    }
                    _ => {}
                }
            }
        }
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1]);

    horizontal[1]
}