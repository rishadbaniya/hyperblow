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

// Stores the previous scroll amount, its usually just increased or decreased by 1
// The main use of this will be to know in which direction the scroll occured in contrast the the
// scroll from previous draw
// "prev" stores the scroll value from previous draw
// "current" stores the scroll value when you are about to draw
pub struct Scroll {
    pub prev: std::cell::Cell<i32>,
    pub current: std::cell::Cell<i32>,
}

impl Scroll {
    // Create a default Scroll instance with no scrolling previously and currently
    fn default() -> Self {
        Scroll {
            prev: std::cell::Cell::new(0),
            current: std::cell::Cell::new(0),
        }
    }
    // Gives the scroll value on the previous draw
    pub fn getPrevious(&self) -> i32 {
        self.prev.get()
    }

    pub fn setPrevious(&self, v: i32) {
        self.prev.set(v);
    }

    // Gives the current scroll value so that you can draw
    pub fn getCurrent(&self) -> i32 {
        self.current.get()
    }

    pub fn setCurrent(&self, v: i32) {
        self.current.set(v);
    }

    pub fn setPrevToCurrent(&self) {
        self.prev.set(self.current.get());
    }
}

pub fn draw<B>(terminal: &mut Terminal<B>) -> Result<()>
where
    B: Backend,
{
    use tui::layout::Direction;
    use tui::style::Color::{Black, Green, Red};

    let mouseOffset = MouseOffset::default();
    let files_scroll = Scroll::default();

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
                        "x : {}, y: {} , Scroll: {}",
                        mouseOffset.get_x(),
                        mouseOffset.get_y(),
                        files_scroll.getCurrent()
                    ))
                    .borders(Borders::ALL)
                    .border_type(tui::widgets::BorderType::Rounded),
                chunks[0],
            );

            files::draw_files(frame, chunks[1], &files_scroll);

            // Sets the previous state of the scroll to the current scroll state
            files_scroll.setPrevToCurrent();
        })?;

        // Blocks the thread until some event is passed
        if let Event = crossterm::event::read()? {
            match Event {
                Event::Key(key) => match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    _ => {}
                },
                Event::Mouse(mouse) => {
                    let updateOffset = || {
                        mouseOffset.set_y(mouse.column);
                        mouseOffset.set_x(mouse.row);
                    };

                    match mouse.kind {
                        MouseEventKind::Down(btn) => {
                            if btn == MouseButton::Left {
                                updateOffset()
                            }
                        }
                        MouseEventKind::ScrollUp => {
                            updateOffset();
                            files_scroll.setCurrent(files_scroll.getCurrent() - 1);
                        }
                        MouseEventKind::ScrollDown => {
                            updateOffset();
                            files_scroll.setCurrent(files_scroll.getCurrent() + 1);
                        }
                        _ => {}
                    };
                }
                _ => {}
            };
        }
    }
}
