use std::io::stdout;
use std::time::Duration;

use crossterm::event::{DisableMouseCapture, EnableMouseCapture, Event, KeyCode};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use tui::backend::{Backend, CrosstermBackend};
use tui::buffer::Cell;
use tui::layout::{Constraint, Layout, Rect};
use tui::terminal::Terminal;
use tui::widgets::{Block, Borders, Row, Table};

use crate::Result;

/// Function that represents the start of the UI rendering of hyperblow

pub fn draw_ui() -> Result<()> {
    // Enabling the raw mode and using alternate screen
    // Note : Any try to invoke println! or any other method related to stdout "fd" won't work after enabling raw mode,
    // TODO : Find a way to print something for debugging purposes or else maile "Terminal.draw"
    // bhitrai😂 print hannu parxa haha
    enable_raw_mode()?;
    let mut stdout = stdout();

    // Enter alternate screen is basically just like opening a vim screen, a complete different
    // universe from your daily terminal screen
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    // Create a backend from Crossterm and connect it with tui-rs Terminal
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Call to draw draw the UI
    draw(&mut terminal)?;

    // Restoring the terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

pub fn draw<B>(terminal: &mut Terminal<B>) -> Result<()>
where
    B: Backend,
{
    use tui::layout::Direction;
    loop {
        terminal.draw(|frame| {
            // Make table to show files
            let table = Table::new(vec![Row::new(vec!["Name", "Download", "Progress"])])
                .widths(&[
                    Constraint::Percentage(50),
                    Constraint::Percentage(10),
                    Constraint::Percentage(40),
                ])
                .style(tui::style::Style::default().fg(tui::style::Color::Red))
                .block(Block::default().borders(Borders::ALL))
                .column_spacing(1);
            // Divide the Rect of Frame vertically in 60% and 30% of the total height
            let chunks = Layout::default()
                .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
                .direction(Direction::Vertical)
                .split(frame.size());
            frame.render_widget(
                Block::default()
                    .title(format!("{:?}", chunks.len()))
                    .borders(Borders::ALL)
                    .border_type(tui::widgets::BorderType::Rounded),
                chunks[0],
            );
            frame.render_widget(table, chunks[1]);
        })?;

        if let Event::Key(key) = crossterm::event::read()? {
            match key.code {
                KeyCode::Char('q') => return Ok(()),
                _ => {}
            }
        }
    }
}
