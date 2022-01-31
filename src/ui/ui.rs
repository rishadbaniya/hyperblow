use std::rc::Rc;
use std::slice::SliceIndex;
use std::time::Duration;

use super::files;
use std::io::stdout;

use crossterm::event::{DisableMouseCapture, EnableMouseCapture, Event, KeyCode};
use crossterm::event::{MouseButton, MouseEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use tui::backend::{Backend, CrosstermBackend};
use tui::layout::{Constraint, Layout, Rect};
use tui::style::{Modifier, Style};
use tui::terminal::Terminal;
use tui::widgets::{Block, Borders, Cell, Row, Table};

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

pub struct FilesState {
    // Start and end index of the range of items that's being drawn
    // Note : Used in scroll to draw the range of data to be drawn
    top_index: std::cell::Cell<u16>,
    bottom_index: std::cell::Cell<u16>,
    // Total size of the files table
    len: std::cell::Cell<u16>,

    // Scroll state
    // if current > previous then we can say user wanted to scroll down
    // if previous < current then we can say user wanted to scroll up
    // The difference is just 1 or -1
    scroll_state_current: std::cell::Cell<i16>,
    scroll_state_previous: std::cell::Cell<i16>,
}

impl FilesState {
    fn new() -> Self {
        FilesState {
            top_index: cell::Cell::new(0),
            bottom_index: cell::Cell::new(0),
            len: cell::Cell::new(0),
            scroll_state_current: cell::Cell::new(0),
            scroll_state_previous: cell::Cell::new(0),
        }
    }

    pub fn set_top_index(&self, v: u16) {
        self.top_index.set(v);
    }
    pub fn get_top_index(&self) -> u16 {
        self.top_index.get()
    }

    pub fn set_bottom_index(&self, v: u16) {
        self.bottom_index.set(v);
    }
    pub fn get_bottom_index(&self) -> u16 {
        self.bottom_index.get()
    }

    pub fn set_scroll_state_current(&self, v: i16) {
        self.scroll_state_current.set(v);
    }
    pub fn get_scroll_state_current(&self) -> i16 {
        self.scroll_state_current.get()
    }
    pub fn set_scroll_state_previous(&self, v: i16) {
        self.scroll_state_previous.set(v);
    }
    pub fn get_scroll_state_previous(&self) -> i16 {
        self.scroll_state_previous.get()
    }
}

pub fn draw<B>(terminal: &mut Terminal<B>) -> Result<()>
where
    B: Backend,
{
    use tui::layout::Direction;

    let mouseOffset = MouseOffset::default();

    let files_state = FilesState::new();

    loop {
        terminal.draw(|frame| {
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
                        mouseOffset.get_x(),
                        mouseOffset.get_y(),
                        files_state.get_scroll_state_previous(),
                        files_state.get_scroll_state_current(),
                        files_state.get_bottom_index(),
                        files_state.get_top_index()
                    ))
                    .borders(Borders::ALL)
                    .border_type(tui::widgets::BorderType::Rounded),
                chunks[0],
            );

            files::draw_files(frame, chunks[1], &files_state);

            // Save the current draw scroll state and use it as previous draw scroll state in
            // next draw
            files_state.set_scroll_state_previous(files_state.get_scroll_state_current());
        })?;

        // Blocks the thread until some event is passed
        if let Event = crossterm::event::read()? {
            match Event {
                Event::Key(key) => match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    _ => {}
                },
                Event::Mouse(mouse) => {
                    match mouse.kind {
                        MouseEventKind::ScrollUp => {
                            files_state.set_scroll_state_current(
                                files_state.get_scroll_state_current() - 1,
                            );
                        }
                        MouseEventKind::ScrollDown => {
                            files_state.set_scroll_state_current(
                                files_state.get_scroll_state_current() + 1,
                            );
                        }
                        _ => {}
                    };
                }
                _ => {}
            };
        }
    }
}
