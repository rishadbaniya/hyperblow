#![allow(non_snake_case)]

//use super::files;
use super::mouse::Mouse;
use super::mouse::MouseEv;
use crossterm::event;
use std::{cell::Cell, io::stdout, ops::Range, rc::Rc, time::Duration};
use tui::style::Modifier;
//use crossterm::execute;
use crate::engine::Engine;
use crossterm::{execute, terminal};
use std::sync::Arc;
use tui::widgets::Gauge;
use tui::{
    backend::{Backend, CrosstermBackend},
    terminal::Terminal,
};
use tui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Color,
    style::Style,
    terminal::Frame,
    text::Spans,
    widgets::{Block, BorderType, Borders, Tabs},
};

use super::tui_state::TUIState;

//// Function that represents the start of the UI rendering of hyperblow
pub async fn draw_ui(engine: Arc<Engine>) -> Result<(), Box<dyn std::error::Error>> {
    // Note : Any try to invoke println! or any other method related to stdout "fd" won't work after enabling raw mode
    terminal::enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, terminal::EnterAlternateScreen, event::EnableMouseCapture)?;
    // Create a backend from Crossterm and connect it with tui-rs Terminal
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Draw the UI
    draw(&mut terminal, engine).await?;

    // Restoring the terminal
    terminal::disable_raw_mode()?;
    execute!(terminal.backend_mut(), terminal::LeaveAlternateScreen, event::DisableMouseCapture)?;
    terminal.show_cursor()?;

    Ok(())
}

pub async fn draw<B: Backend>(terminal: &mut Terminal<B>, engine: Arc<Engine>) -> crossterm::Result<()> {
    let state = Rc::new(TUIState::new(engine));
    loop {
        terminal.draw(|frame| {
            //Divide the Rect of Frame vertically in 60% and 30% of the total height
            let chunks = Layout::default()
                .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
                .direction(Direction::Vertical)
                .split(frame.size());

            drawTorrentsSection(frame, chunks[0]);
            drawTabsSection(frame, chunks[1], state.clone());
        })?;

        // Waits for upto 200 ms for some event to occure before moving on the rendering the new frame,
        // as soon as it gets an event, it moves on to rendering a new frame, and doesn't entirely
        // wait 200ms
        if event::poll(Duration::from_millis(200))? {
            match event::read()? {
                event::Event::Key(key) => match key.code {
                    event::KeyCode::Char('q') => return Ok(()),
                    event::KeyCode::Tab => {
                        state.increment_tab_index();
                    }
                    _ => {}
                },

                event::Event::Mouse(mouse_event) => {
                    match mouse_event.kind {
                        event::MouseEventKind::Down(btn) => {
                            if btn == event::MouseButton::Left {
                                state.mouse.set_x(mouse_event.column);
                                state.mouse.set_y(mouse_event.row);
                                state.mouse.set_event(MouseEv::Clicked);
                                //                 // TODO : Write a code such that file_state.buttonClick is only invoked
                                //                 // when the button was clicked on one of the component of File Tab
                                //                 filesState.buttonClick(mouse_offset.get_x(), mouse_offset.get_y());
                            }
                        }
                        event::MouseEventKind::ScrollUp => {}
                        event::MouseEventKind::ScrollDown => {}
                        _ => {}
                    };
                }
                _ => {}
            };
        }
        state.refresh().await;
    }
}

// Top Section :
//      Dispaly region to render all the high level state of those torrents such as
//      - Name, Bytes, Speed Out, Speed In, Progress, Pause/Resume -> To be extracted from the TorrentHandle within the Engine
fn drawTorrentsSection<B: Backend>(frame: &mut Frame<B>, area: Rect) {
    let torrents_section = Block::default()
        .title(" Torrents ")
        .title_alignment(tui::layout::Alignment::Left)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);
    frame.render_widget(torrents_section, area);
}
//struct TabTitle {
//spans: Vec<Spans<'static>>,
//x_range: Vec<Range<usize>>,
//}

// Bottom Section :
//      Dispaly region to render all the high level state of those torrents such as
//      - Name, Bytes, Speed Out, Speed In, Progress, Pause/Resume -> To be extracted from the TorrentHandle within the Engine
// TODO : Add support for mouse in Tabs somehow, from the library itself there is no support of mouse in tabs
//
fn drawTabsSection<B: Backend>(frame: &mut Frame<B>, area: Rect, state: Rc<TUIState>) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
        .split(area);

    let titles: Vec<Spans> = ["Details", "Bandwidth", "Files", "Trackers", "Peers", "Pieces"]
        .iter()
        .cloned()
        .map(Spans::from)
        .collect();

    state.set_max_tab_index(titles.len() - 1);

    let widget_tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title("Tabs"))
        .select(state.tab_index())
        .style(Style::default().fg(Color::Cyan))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD).bg(Color::Black));

    frame.render_widget(widget_tabs, chunks[0]);

    match *state.tabs_section.borrow() {
        super::tui_state::TabsSection::Details(ref details) => details.renderWidget(frame, chunks[1]),
        _ => {
            let widget_border = Block::default().borders(Borders::ALL).border_type(BorderType::Rounded);
            frame.render_widget(widget_border, chunks[1]);
        }
    };

    //frame.render_widget(gauge, children_chunks[0]);
}
