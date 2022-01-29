use std::borrow::BorrowMut;
use std::fmt::format;
use std::io::stdout;
use std::time::Duration;

use crossterm::event::{DisableMouseCapture, EnableMouseCapture, Event};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use titik::{Checkbox, FlexBox, Renderer, Widget};
use tui::backend::{Backend, CrosstermBackend};
use tui::layout::{Layout, Rect};
use tui::terminal::Terminal;
use tui::widgets::{Block, Borders};

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
    terminal.draw(|frame| {
        frame.render_widget(
            Block::default()
                .title("Files")
                .borders(Borders::ALL)
                .border_type(tui::widgets::BorderType::Rounded),
            Rect::new(0, 0, 40, 20),
        );

        let size = frame.size();
        let m = format!("({},{})", size.width, size.height);
        frame.render_widget(Block::default().title(m), Rect::new(2, 2, 20, 4));
    })?;

    // Sleeping the thread for 5 secs, so that i can see wtf is getting printed
    // using terminal.draw on the alternate screen
    if cfg!(debug_assertions) {
        std::thread::sleep(Duration::from_millis(2000));
    }

    Ok(())
}
