#![allow(non_snake_case)]

use super::files;
use std::io::stdout;

use crossterm::event::{DisableMouseCapture, EnableMouseCapture, Event, KeyCode};
use crossterm::event::{MouseButton, MouseEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use tui::backend::{Backend, CrosstermBackend};
use tui::layout::{Constraint, Layout};
use tui::terminal::Terminal;
use tui::widgets::{Block, Borders};

use crate::Result;
use std::sync::{Arc, Mutex};

/// Function that represents the start of the UI rendering of hyperblow
pub fn draw_ui(fileState: Arc<Mutex<files::FilesState>>) -> Result<()> {
    // Enabling the raw mode and using alternate screen
    // Note : Any try to invoke println! or any other method related to stdout "fd" won't work after enabling raw mode
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    // Create a backend from Crossterm and connect it with tui-rs Terminal
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Call to draw draw the UI
    draw(&mut terminal, fileState)?;

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

use std::cell;

// Struct that stores the offset of mouse everytime we move the cursor
// Note : Used to store the mouse offset as a global state
struct MouseOffset {
    // Offset in (x, y) format
    offset: (cell::Cell<u16>, cell::Cell<u16>),
}

impl MouseOffset {
    // Used to create MouseOffset instance initially
    fn default() -> Self {
        Self {
            offset: (cell::Cell::new(0), cell::Cell::new(0)),
        }
    }

    fn get_x(&self) -> u16 {
        self.offset.0.get()
    }

    fn get_y(&self) -> u16 {
        self.offset.1.get()
    }

    fn set_x(&self, x: u16) {
        self.offset.0.set(x);
    }

    fn set_y(&self, y: u16) {
        self.offset.1.set(y);
    }
}

pub fn draw<B>(terminal: &mut Terminal<B>, filesState: Arc<Mutex<files::FilesState>>) -> Result<()>
where
    B: Backend,
{
    use tui::layout::Direction;

    let mouse_offset = MouseOffset::default();

    loop {
        terminal.draw(|frame| {
            let mut filesState = filesState.lock().unwrap();
            // Divide the Rect of Frame vertically in 60% and 30% of the total height
            let chunks = Layout::default()
                .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
                .direction(Direction::Vertical)
                .split(frame.size());

            //Bottom Section
            frame.render_widget(
                Block::default()
                    .title(format!(
                        "x : {}, y: {} , Previous : {}, Current : {} | Bottom : {}, Top : {}",
                        mouse_offset.get_x(),
                        mouse_offset.get_y(),
                        filesState.get_scroll_state_previous(),
                        filesState.get_scroll_state_current(),
                        filesState.get_bottom_index(),
                        filesState.get_top_index()
                    ))
                    .borders(Borders::ALL)
                    .border_type(tui::widgets::BorderType::Rounded),
                chunks[0],
            );

            files::draw_files(frame, chunks[1], &mut filesState);

            // Save the current draw scroll state and use it as previous draw scroll state in
            // next draw
            filesState.set_scroll_state_previous(filesState.get_scroll_state_current());
        })?;

        // Blocks the thread until some event is passed
        match crossterm::event::read()? {
            Event::Key(key) => match key.code {
                KeyCode::Char('q') => return Ok(()),
                _ => {}
            },
            Event::Mouse(mouse) => {
                let mut filesState = filesState.lock().unwrap();
                match mouse.kind {
                    MouseEventKind::Down(btn) => {
                        if btn == MouseButton::Left {
                            mouse_offset.set_x(mouse.column);
                            mouse_offset.set_y(mouse.row);

                            // TODO : Write a code such that file_state.buttonClick is only invoked
                            // when the button was clicked on one of the component of File Tab
                            filesState.buttonClick(mouse_offset.get_x(), mouse_offset.get_y());
                        }
                    }
                    MouseEventKind::ScrollUp => filesState.scrollGoingUp(),
                    MouseEventKind::ScrollDown => filesState.scrollGoingDown(),
                    _ => {}
                };
            }
            _ => {}
        };
    }
}
