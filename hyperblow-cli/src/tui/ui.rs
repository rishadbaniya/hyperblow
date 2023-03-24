#![allow(non_snake_case)]

use super::{
    super::engine::Engine,
    mouse::MouseEv,
    sections::{
        tabs_section::{
            bandwidth_tab::BandwidthTab, details_tab::DetailsTab, files_tab::FilesTab, peers_tab::PeersTab,
            pieces_tab::PiecesTab, trackers_tab::TrackersTab,
        },
        torrents_section::TorrentsSection,
    },
    tui_state::{TUIState, Tab},
};
use crossterm::{event, execute, terminal};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    terminal::{Frame, Terminal},
    text::Spans,
    widgets::{Block, BorderType, Borders, Tabs},
};
use std::{io::stdout, rc::Rc, sync::Arc, time::Duration};

/// Draws the ui by setting up the raw mode and calling the other draw method
pub fn draw_ui(engine: Arc<Engine>) -> Result<(), Box<dyn std::error::Error>> {
    terminal::enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, terminal::EnterAlternateScreen, event::EnableMouseCapture)?;

    // Create a backend from Crossterm and connect it with tui-rs Terminal
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Draw the UI
    draw(&mut terminal, engine)?;

    // Restoring the terminal
    terminal::disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        terminal::LeaveAlternateScreen,
        event::DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

// Handles drawing items
fn draw<B: Backend>(terminal: &mut Terminal<B>, engine: Arc<Engine>) -> crossterm::Result<()> {
    let state = Rc::new(TUIState::new(engine));

    loop {
        terminal.draw(|frame| {
            //let x = local.spawn_local(async { 10 });
            //Divide the Rect of Frame vertically in 60% and 30% of the total height
            let chunks = Layout::default()
                .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
                .direction(Direction::Vertical)
                .split(frame.size());
            TorrentsSection::draw(frame, chunks[0], state.clone());
            drawTabsSection(frame, chunks[1], state.clone());
        })?;

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
        //state.refresh().await;
    }
}

// Top Section :
//      Dispaly region to render all the high level state of those torrents such as
//      - Name, Bytes, Speed Out, Speed In, Progress, Pause/Resume -> To be extracted from the TorrentHandle within the Engine
fn drawTorrentsSection<B: Backend>(frame: &mut Frame<B>, area: Rect) {
    let torrents_section = Block::default()
        .title(" Torrents ")
        .title_alignment(Alignment::Left)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);
    frame.render_widget(torrents_section, area);
}

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
        .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded))
        .select(state.tab_index())
        .style(Style::default().fg(Color::Cyan))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD).bg(Color::Black));

    // Draws the Tabs
    frame.render_widget(widget_tabs, chunks[0]);

    // Reads the currently selected tab from the state and then draws that Tab
    match *state.tab.borrow() {
        Tab::Details => DetailsTab::draw(frame, chunks[1], state.clone()),
        Tab::Bandwidth => BandwidthTab::draw(frame, chunks[1], state.clone()),
        Tab::Files => FilesTab::draw(frame, chunks[1], state.clone()),
        Tab::Trackers => TrackersTab::draw(frame, chunks[1], state.clone()),
        Tab::Peers => PeersTab::draw(frame, chunks[1], state.clone()),
        Tab::Pieces => PiecesTab::draw(frame, chunks[1], state.clone()),
        Tab::None => {
            // Draws nothing
        }
    };
}
