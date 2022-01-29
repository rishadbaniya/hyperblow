use std::io::stdout;

use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use tui::backend::{Backend, CrosstermBackend};
use tui::terminal::Terminal;

use crate::Result;

pub fn draw_ui() -> Result<()> {
    // Enabling the raw mode and using alternate screen
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Call to start the app and draw the UI

    start_and_draw(&mut terminal)?;

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

pub fn start_and_draw<B>(terminal: &mut Terminal<B>) -> Result<()>
where
    B: Backend,
{
    //terminal.draw(|frame| {});

    if cfg!(debug_assertions) {
        println!("Yo! this is working");
    }

    Ok(())
}
