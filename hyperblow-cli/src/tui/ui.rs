#![allow(non_snake_case)]

use super::{
    super::engine::Engine,
    mouse::MouseEv,
    sections::{
        tabs_section::{
            bandwidth_tab::BandwidthTab, details_tab::DetailsTab, files_tab::FilesTab, peers_tab::PeersTab, pieces_tab::PiecesTab,
            trackers_tab::TrackersTab,
        },
        torrents_section::TorrentsSection,
    },
    tui_state::{TUIState, Tab},
};
use crossterm::{event, execute, terminal};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout, Position, Rect, Size},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, BorderType, Borders, Tabs},
    Frame, Terminal,
};
use std::{io::stdout, rc::Rc, sync::Arc, time::Duration};

const TAB_TITLES: [&str; 6] = ["Details", "Bandwidth", "Files", "Trackers", "Peers", "Pieces"];
const TAB_PADDING_WIDTH: u16 = 1;
const TAB_DIVIDER_WIDTH: u16 = 1;

#[derive(Clone, Copy, Debug)]
struct AppLayout {
    torrents_section: Rect,
    torrent_rows: Rect,
    tabs_section: Rect,
    tab_bar: Rect,
}

/// Draws the ui by setting up the raw mode and calling the other draw method
pub fn draw_ui(engine: Arc<Engine>) -> Result<(), Box<dyn std::error::Error>> {
    terminal::enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, terminal::EnterAlternateScreen, event::EnableMouseCapture)?;

    // Create a backend from Crossterm and connect it with tui-rs Terminal
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let draw_result = draw(&mut terminal, engine);

    let cleanup_result = (|| -> Result<(), Box<dyn std::error::Error>> {
        terminal::disable_raw_mode()?;
        execute!(terminal.backend_mut(), terminal::LeaveAlternateScreen, event::DisableMouseCapture)?;
        terminal.show_cursor()?;
        Ok(())
    })();

    match (draw_result, cleanup_result) {
        (Err(error), _) => Err(error),
        (Ok(()), Err(error)) => Err(error),
        (Ok(()), Ok(())) => Ok(()),
    }
}

// Handles drawing items
fn draw<B: Backend>(terminal: &mut Terminal<B>, engine: Arc<Engine>) -> Result<(), Box<dyn std::error::Error>>
where
    B::Error: 'static,
{
    let state = Rc::new(TUIState::new(engine));

    loop {
        terminal.draw(|frame| render_app_frame(frame, state.clone()))?;

        if event::poll(Duration::from_millis(200))? {
            let terminal_area = terminal_area_from_size(terminal.size()?);
            match event::read()? {
                event::Event::Key(key) => match key.code {
                    event::KeyCode::Char('q') => return Ok(()),
                    event::KeyCode::Tab => {
                        state.increment_tab_index();
                    }
                    event::KeyCode::BackTab => {
                        state.decrement_tab_index();
                    }
                    event::KeyCode::Down => {
                        state.increment_torrent_index();
                    }
                    event::KeyCode::Up => {
                        state.decrement_torrent_index();
                    }
                    _ => {}
                },

                event::Event::Mouse(mouse_event) => {
                    handle_mouse_event(mouse_event, terminal_area, state.as_ref());
                }
                _ => {}
            };
        }
        //state.refresh().await;
    }
}

fn render_app_frame(frame: &mut Frame, state: Rc<TUIState>) {
    let layout = app_layout(frame.area());
    TorrentsSection::draw(frame, layout.torrents_section, state.clone());
    drawTabsSection(frame, layout.tabs_section, state);
}

fn app_layout(area: Rect) -> AppLayout {
    let chunks = Layout::default()
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .direction(Direction::Vertical)
        .split(area);

    let torrents_inner = inset_rect(chunks[0], 2, 2);
    let torrent_chunks = Layout::default()
        .constraints([Constraint::Length(2), Constraint::Min(0)])
        .split(torrents_inner);
    let tabs_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
        .split(chunks[1]);

    AppLayout {
        torrents_section: chunks[0],
        torrent_rows: torrent_chunks[1],
        tabs_section: chunks[1],
        tab_bar: tabs_chunks[0],
    }
}

fn terminal_area_from_size(size: Size) -> Rect {
    Rect::new(0, 0, size.width, size.height)
}

fn inset_rect(area: Rect, horizontal: u16, vertical: u16) -> Rect {
    Rect {
        x: area.x.saturating_add(horizontal),
        y: area.y.saturating_add(vertical),
        width: area.width.saturating_sub(horizontal.saturating_mul(2)),
        height: area.height.saturating_sub(vertical.saturating_mul(2)),
    }
}

fn handle_mouse_event(mouse_event: event::MouseEvent, terminal_area: Rect, state: &TUIState) {
    state.mouse.set_x(mouse_event.column);
    state.mouse.set_y(mouse_event.row);

    let layout = app_layout(terminal_area);
    let position = Position {
        x: mouse_event.column,
        y: mouse_event.row,
    };

    match mouse_event.kind {
        event::MouseEventKind::Down(event::MouseButton::Left) => {
            state.mouse.set_event(MouseEv::Clicked);
            if select_tab_at(layout.tab_bar, position, state) {
                return;
            }
            select_torrent_at(layout.torrent_rows, position, state);
        }
        event::MouseEventKind::Drag(event::MouseButton::Left) => {
            state.mouse.set_event(MouseEv::Clicked);
            select_torrent_at(layout.torrent_rows, position, state);
        }
        event::MouseEventKind::Up(_) => {
            state.mouse.set_event(MouseEv::NotClicked);
        }
        event::MouseEventKind::ScrollUp => {
            if layout.torrents_section.contains(position) {
                state.decrement_torrent_index();
            } else if layout.tab_bar.contains(position) {
                state.decrement_tab_index();
            }
        }
        event::MouseEventKind::ScrollDown => {
            if layout.torrents_section.contains(position) {
                state.increment_torrent_index();
            } else if layout.tab_bar.contains(position) {
                state.increment_tab_index();
            }
        }
        event::MouseEventKind::ScrollLeft => {
            if layout.tabs_section.contains(position) {
                state.decrement_tab_index();
            }
        }
        event::MouseEventKind::ScrollRight => {
            if layout.tabs_section.contains(position) {
                state.increment_tab_index();
            }
        }
        event::MouseEventKind::Down(_) | event::MouseEventKind::Drag(_) | event::MouseEventKind::Moved => {}
    }
}

fn select_tab_at(tab_bar: Rect, position: Position, state: &TUIState) -> bool {
    let Some(tab_index) = tab_index_at(tab_bar, position) else {
        return false;
    };
    state.set_tab_index(tab_index);
    true
}

fn select_torrent_at(torrent_rows: Rect, position: Position, state: &TUIState) -> bool {
    let Some(torrent_index) = torrent_index_at(torrent_rows, position) else {
        return false;
    };

    let Some(torrent_count) = state.engine.torrent_snapshot().map(|torrents| torrents.len()) else {
        return false;
    };
    if torrent_index >= torrent_count {
        return false;
    }

    state.set_max_torrent_index(torrent_count.saturating_sub(1));
    state.set_torrent_index(torrent_index);
    true
}

fn tab_index_at(tab_bar: Rect, position: Position) -> Option<usize> {
    let tab_content = inset_rect(tab_bar, 1, 1);
    if !tab_content.contains(position) {
        return None;
    }

    let mut cursor_x = tab_content.x;
    for (index, title) in TAB_TITLES.iter().enumerate() {
        let title_width = title.len() as u16;
        let item_width = TAB_PADDING_WIDTH
            .saturating_mul(2)
            .saturating_add(title_width)
            .min(tab_content.right().saturating_sub(cursor_x));
        let item_end = cursor_x.saturating_add(item_width);
        if position.x >= cursor_x && position.x < item_end {
            return Some(index);
        }
        cursor_x = item_end;

        if index + 1 < TAB_TITLES.len() {
            let divider_end = cursor_x.saturating_add(TAB_DIVIDER_WIDTH).min(tab_content.right());
            if position.x >= cursor_x && position.x < divider_end {
                return Some(index);
            }
            cursor_x = divider_end;
        }
    }

    None
}

fn torrent_index_at(torrent_rows: Rect, position: Position) -> Option<usize> {
    if torrent_rows.contains(position) {
        Some(position.y.saturating_sub(torrent_rows.y) as usize)
    } else {
        None
    }
}

// Top Section :
//      Dispaly region to render all the high level state of those torrents such as
//      - Name, Bytes, Speed Out, Speed In, Progress, Pause/Resume -> To be extracted from the TorrentHandle within the Engine
fn drawTorrentsSection(frame: &mut Frame, area: Rect) {
    let torrents_section = Block::default()
        .title(" Torrents ")
        .title_alignment(Alignment::Left)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);
    frame.render_widget(torrents_section, area);
}

fn drawTabsSection(frame: &mut Frame, area: Rect, state: Rc<TUIState>) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
        .split(area);

    let titles: Vec<Line> = TAB_TITLES.iter().cloned().map(Line::from).collect();

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
    };
}

#[cfg(test)]
mod tests {
    use super::{
        app_layout, handle_mouse_event, inset_rect, render_app_frame, tab_index_at, torrent_index_at, TAB_DIVIDER_WIDTH, TAB_PADDING_WIDTH,
        TAB_TITLES,
    };
    use crate::{
        engine::{Engine, TorrentSource},
        tui::tui_state::TUIState,
    };
    use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
    use ratatui::{
        backend::TestBackend,
        layout::{Position, Rect},
        Terminal,
    };
    use std::rc::Rc;

    #[test]
    fn renders_empty_app_without_panicking() {
        let backend = TestBackend::new(100, 32);
        let mut terminal = Terminal::new(backend).expect("test backend should initialize");
        let state = Rc::new(TUIState::new(Engine::new()));

        terminal
            .draw(|frame| render_app_frame(frame, state.clone()))
            .expect("empty app should render");

        let rendered = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(rendered.contains("Hyperblow"));
        assert!(rendered.contains("No torrents loaded"));
    }

    #[test]
    fn finds_clicked_tab_from_rendered_tab_bar() {
        let tab_bar = Rect::new(0, 20, 100, 3);

        assert_eq!(tab_index_at(tab_bar, tab_position(tab_bar, 0)), Some(0));
        assert_eq!(tab_index_at(tab_bar, tab_position(tab_bar, 2)), Some(2));
        assert_eq!(tab_index_at(tab_bar, Position { x: 99, y: 21 }), None);
    }

    #[test]
    fn maps_torrent_rows_to_visible_indexes() {
        let torrent_rows = Rect::new(2, 4, 80, 8);

        assert_eq!(torrent_index_at(torrent_rows, Position { x: 3, y: 4 }), Some(0));
        assert_eq!(torrent_index_at(torrent_rows, Position { x: 3, y: 7 }), Some(3));
        assert_eq!(torrent_index_at(torrent_rows, Position { x: 3, y: 12 }), None);
    }

    #[test]
    fn mouse_click_selects_tab() {
        let terminal_area = Rect::new(0, 0, 100, 32);
        let layout = app_layout(terminal_area);
        let state = TUIState::new(Engine::new());
        state.set_max_tab_index(TAB_TITLES.len() - 1);

        let position = tab_position(layout.tab_bar, 3);
        handle_mouse_event(left_click(position), terminal_area, &state);

        assert_eq!(state.tab_index(), 3);
        assert_eq!(state.mouse.get_x(), position.x);
        assert_eq!(state.mouse.get_y(), position.y);
    }

    #[test]
    fn mouse_click_selects_torrent_row() {
        let engine = Engine::new();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("test runtime should initialize");
        runtime.block_on(async {
            engine
                .spawn(TorrentSource::MagnetURI(
                    "magnet:?xt=urn:btih:08ada5a7a6183aae1e09d831df6748d566095a10&dn=First".to_string(),
                ))
                .await
                .expect("first magnet should spawn");
            engine
                .spawn(TorrentSource::MagnetURI(
                    "magnet:?xt=urn:btih:08ada5a7a6183aae1e09d831df6748d566095a11&dn=Second".to_string(),
                ))
                .await
                .expect("second magnet should spawn");
        });

        let terminal_area = Rect::new(0, 0, 100, 32);
        let layout = app_layout(terminal_area);
        let state = TUIState::new(engine);
        state.set_max_torrent_index(1);

        handle_mouse_event(
            left_click(Position {
                x: layout.torrent_rows.x + 1,
                y: layout.torrent_rows.y + 1,
            }),
            terminal_area,
            &state,
        );

        assert_eq!(state.torrent_index(), 1);
    }

    #[test]
    fn mouse_wheel_changes_selection_in_context() {
        let terminal_area = Rect::new(0, 0, 100, 32);
        let layout = app_layout(terminal_area);
        let state = TUIState::new(Engine::new());
        state.set_max_tab_index(TAB_TITLES.len() - 1);
        state.set_tab_index(1);
        state.set_max_torrent_index(2);
        state.set_torrent_index(1);

        handle_mouse_event(
            mouse_event(
                MouseEventKind::ScrollDown,
                Position {
                    x: layout.torrents_section.x + 1,
                    y: layout.torrents_section.y + 1,
                },
            ),
            terminal_area,
            &state,
        );
        assert_eq!(state.torrent_index(), 2);

        handle_mouse_event(
            mouse_event(MouseEventKind::ScrollUp, tab_position(layout.tab_bar, 1)),
            terminal_area,
            &state,
        );
        assert_eq!(state.tab_index(), 0);
    }

    fn tab_position(tab_bar: Rect, tab_index: usize) -> Position {
        let tab_content = inset_rect(tab_bar, 1, 1);
        let x_offset = TAB_TITLES
            .iter()
            .take(tab_index)
            .map(|title| TAB_PADDING_WIDTH * 2 + title.len() as u16 + TAB_DIVIDER_WIDTH)
            .sum::<u16>();

        Position {
            x: tab_content.x + x_offset + TAB_PADDING_WIDTH,
            y: tab_content.y,
        }
    }

    fn left_click(position: Position) -> MouseEvent {
        mouse_event(MouseEventKind::Down(MouseButton::Left), position)
    }

    fn mouse_event(kind: MouseEventKind, position: Position) -> MouseEvent {
        MouseEvent {
            kind,
            column: position.x,
            row: position.y,
            modifiers: KeyModifiers::empty(),
        }
    }
}
