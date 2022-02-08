#![allow(non_snake_case)]

use super::files;
use std::io::stdout;
use std::time::Duration;

use super::mouse::MouseOffset;
use crossterm::event::{poll, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, MouseButton, MouseEventKind};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use tui::backend::{Backend, CrosstermBackend};
use tui::layout::{Constraint, Direction, Layout, Rect};
use tui::style::Style;
use tui::terminal::Terminal;
use tui::widgets::{Block, Borders, Gauge};

use crate::details::Details;
use crate::{details, Result};
use std::sync::{Arc, Mutex};
use tokio::sync::Mutex as TokioMutex;

// Function that represents the start of the UI rendering of hyperblow
pub fn draw_ui(fileState: Arc<Mutex<files::FilesState>>, details: Arc<TokioMutex<Details>>) -> Result<()> {
    // Note : Any try to invoke println! or any other method related to stdout "fd" won't work after enabling raw mode
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    // Create a backend from Crossterm and connect it with tui-rs Terminal
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Call to draw draw the UI
    draw(&mut terminal, fileState, details)?;

    // Restoring the terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    Ok(())
}

pub fn draw<B>(terminal: &mut Terminal<B>, filesState: Arc<Mutex<files::FilesState>>, details: Arc<TokioMutex<Details>>) -> Result<()>
where
    B: Backend,
{
    let mouse_offset = MouseOffset::default();

    loop {
        terminal.draw(|frame| {
            let mut filesState = filesState.lock().unwrap();

            // Divide the Rect of Frame vertically in 60% and 30% of the total height
            let chunks = Layout::default()
                .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
                .direction(Direction::Vertical)
                .split(frame.size());

            //Top Section
            let details_section = (
                Block::default()
                    .title(" Details ")
                    .title_alignment(tui::layout::Alignment::Center)
                    .borders(Borders::ALL)
                    .border_type(tui::widgets::BorderType::Rounded),
                chunks[0],
            );

            //Torrent name inside of Top Section
            let torrent_name = (
                Block::default().title(format!("Name : {}", filesState.name)),
                Rect::new(1, 2, frame.size().width - 2, 1),
            );

            let downloadProgressBar = (
                Gauge::default()
                    .block(Block::default().title(format!(
                        "Downloading : 10.5 Mb/ 2500 Mb || Down Speed : {} Mb/s || Up Speed : {} Mb/s",
                        3.1, 2.1
                    )))
                    .gauge_style(Style::default().fg(tui::style::Color::Green))
                    .percent(0),
                Rect::new(1, 6, frame.size().width - 2, 2),
            );

            frame.render_widget(details_section.0, details_section.1);
            frame.render_widget(torrent_name.0, torrent_name.1);
            frame.render_widget(downloadProgressBar.0, downloadProgressBar.1);
            files::draw_files(frame, chunks[1], &mut filesState);
            draw_details(frame, chunks[0], details.clone());
            // Save the current draw scroll state and use it as previous draw scroll state in
            // next draw
            filesState.set_scroll_state_previous(filesState.get_scroll_state_current());
        })?;

        // Waits for at least 200ms for some event to occur before moving on
        if poll(Duration::from_millis(200))? {
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
}

use tui::terminal::Frame;

// Draws Details section
pub fn draw_details<B: Backend>(frame: &mut Frame<B>, size: Rect, details: Arc<TokioMutex<Details>>) {
    let details_lock = details.blocking_lock();
    let info_hash = details_lock.info_hash.as_ref().unwrap();
    let name = details_lock.name.as_ref().unwrap();

    let name = (Block::default().title(format!("Name : {}", name)), Rect::new(1, 2, frame.size().width - 2, 1));

    let info_hash = (
        Block::default().title(format!("Info Hash : {:?}", info_hash)),
        Rect::new(1, 4, frame.size().width - 2, 1),
    );

    frame.render_widget(name.0, name.1);
    frame.render_widget(info_hash.0, info_hash.1);
}
