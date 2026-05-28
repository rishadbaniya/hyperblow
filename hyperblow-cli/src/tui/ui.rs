#![allow(non_snake_case)]

use super::{
    super::engine::Engine,
    command::{CommandAction, CommandExecutionResult, CommandExecutor, CommandParser, CommandSuggester},
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
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Tabs, Wrap},
    Frame, Terminal,
};
use std::{
    io::stdout,
    rc::Rc,
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc,
    },
    time::Duration,
};
use tracing::{debug, info};

const TAB_TITLES: [&str; 6] = ["Details", "Bandwidth", "Files", "Trackers", "Peers", "Pieces"];
const TAB_PADDING_WIDTH: u16 = 1;
const TAB_DIVIDER_WIDTH: u16 = 1;

#[derive(Clone, Copy, Debug)]
struct AppLayout {
    torrents_section: Rect,
    torrent_rows: Rect,
    tabs_section: Rect,
    tab_bar: Rect,
    tab_content_rows: Rect,
}

impl AppLayout {
    fn new(area: Rect) -> Self {
        let chunks = Layout::default()
            .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
            .direction(Direction::Vertical)
            .split(area);

        let torrents_inner = RectMath::inset(chunks[0], 2, 2);
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
            tab_content_rows: tabs_chunks[1],
        }
    }
}

struct RectMath;

impl RectMath {
    fn from_size(size: Size) -> Rect {
        Rect::new(0, 0, size.width, size.height)
    }

    fn inset(area: Rect, horizontal: u16, vertical: u16) -> Rect {
        Rect {
            x: area.x.saturating_add(horizontal),
            y: area.y.saturating_add(vertical),
            width: area.width.saturating_sub(horizontal.saturating_mul(2)),
            height: area.height.saturating_sub(vertical.saturating_mul(2)),
        }
    }
}

pub struct TuiApplication;

impl TuiApplication {
    pub fn run_ui(engine: Arc<Engine>) -> Result<(), Box<dyn std::error::Error>> {
        info!("entering TUI");
        terminal::enable_raw_mode()?;
        let mut stdout = stdout();
        execute!(stdout, terminal::EnterAlternateScreen, event::EnableMouseCapture)?;
        debug!("TUI terminal modes enabled");

        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let draw_result = Self::run(&mut terminal, engine);

        let cleanup_result = (|| -> Result<(), Box<dyn std::error::Error>> {
            terminal::disable_raw_mode()?;
            execute!(terminal.backend_mut(), terminal::LeaveAlternateScreen, event::DisableMouseCapture)?;
            terminal.show_cursor()?;
            debug!("TUI terminal modes restored");
            Ok(())
        })();

        match (draw_result, cleanup_result) {
            (Err(error), _) => Err(error),
            (Ok(()), Err(error)) => Err(error),
            (Ok(()), Ok(())) => Ok(()),
        }
    }

    fn run<B: Backend>(terminal: &mut Terminal<B>, engine: Arc<Engine>) -> Result<(), Box<dyn std::error::Error>>
    where
        B::Error: 'static,
    {
        let state = Rc::new(TUIState::new(engine));
        let (command_result_sender, command_result_receiver) = mpsc::channel::<CommandExecutionResult>();

        loop {
            CommandController::drain_results(state.as_ref(), &command_result_receiver);
            terminal.draw(|frame| AppRenderer::render(frame, state.clone()))?;

            if event::poll(Duration::from_millis(200))? {
                let terminal_area = RectMath::from_size(terminal.size()?);
                match event::read()? {
                    event::Event::Key(key) => {
                        if state.is_command_mode() {
                            if CommandController::handle_key(key, state.as_ref(), command_result_sender.clone()) {
                                return Ok(());
                            }
                            continue;
                        }

                        match key.code {
                            event::KeyCode::Char(':') => {
                                state.clear_command_input();
                                state.enter_command_mode();
                                CommandController::refresh_suggestions(state.as_ref());
                                debug!("entered command mode");
                            }
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
                        }
                    }

                    event::Event::Mouse(mouse_event) => {
                        MouseController::handle_event(mouse_event, terminal_area, state.as_ref());
                    }
                    _ => {}
                };
            }
        }
    }
}

struct CommandController;

impl CommandController {
    fn drain_results(state: &TUIState, receiver: &Receiver<CommandExecutionResult>) {
        while let Ok(result) = receiver.try_recv() {
            state.decrement_pending_commands();
            match result {
                CommandExecutionResult::Loaded { message } => {
                    info!(message = %message, "command result loaded");
                    state.clear_command_input();
                    state.exit_command_mode();
                    state.set_command_feedback(message, false);
                }
                CommandExecutionResult::Failed { input, message } => {
                    debug!(message = %message, "command result failed");
                    state.set_command_input(input);
                    state.enter_command_mode();
                    state.set_command_feedback(message, true);
                }
            }
        }
    }

    fn handle_key(key: event::KeyEvent, state: &TUIState, command_result_sender: Sender<CommandExecutionResult>) -> bool {
        match key.code {
            event::KeyCode::Esc => {
                Self::cancel(state);
            }
            event::KeyCode::Char('c') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                Self::cancel(state);
            }
            event::KeyCode::Enter => {
                return Self::submit(state, command_result_sender);
            }
            event::KeyCode::Backspace => {
                state.pop_command_char();
                Self::refresh_suggestions(state);
            }
            event::KeyCode::Tab => {
                if let Some(suggestion) = state.selected_command_suggestion() {
                    state.set_command_input(suggestion);
                    Self::refresh_suggestions(state);
                }
            }
            event::KeyCode::Down => {
                state.increment_command_suggestion_index();
            }
            event::KeyCode::Up => {
                state.decrement_command_suggestion_index();
            }
            event::KeyCode::Char(character) if key.modifiers.is_empty() || key.modifiers == event::KeyModifiers::SHIFT => {
                state.push_command_char(character);
                Self::refresh_suggestions(state);
            }
            _ => {}
        }
        false
    }

    fn cancel(state: &TUIState) {
        state.clear_command_input();
        state.set_command_suggestions(Vec::new());
        state.clear_command_feedback();
        state.exit_command_mode();
        debug!("cancelled command mode");
    }

    fn submit(state: &TUIState, command_result_sender: Sender<CommandExecutionResult>) -> bool {
        if state.has_pending_commands() {
            state.set_command_feedback("A torrent open command is already running".to_string(), true);
            return false;
        }

        let input = state.command_input();
        debug!(input_length = input.len(), "submitting command input");
        match CommandParser::parse(&input) {
            Ok(CommandAction::Quit) => {
                info!("quit requested from command mode");
                state.clear_command_input();
                state.exit_command_mode();
                true
            }
            Ok(action) => {
                state.increment_pending_commands();
                state.exit_command_mode();
                state.set_command_feedback(CommandExecutor::pending_message(&action), false);
                CommandExecutor::spawn(action, input, state.engine.clone(), command_result_sender);
                false
            }
            Err(error) => {
                state.set_command_feedback(error.to_string(), true);
                Self::refresh_suggestions(state);
                false
            }
        }
    }

    fn refresh_suggestions(state: &TUIState) {
        state.set_command_suggestions(CommandSuggester::suggestions(&state.command_input(), 8));
    }
}

struct AppRenderer;

impl AppRenderer {
    fn render(frame: &mut Frame, state: Rc<TUIState>) {
        let is_empty = state.engine.torrent_snapshot().map(|torrents| torrents.is_empty()).unwrap_or(false);
        if is_empty {
            Self::draw_start_screen(frame, frame.area(), state.clone());
        } else {
            let layout = AppLayout::new(frame.area());
            TorrentsSection::draw(frame, layout.torrents_section, state.clone());
            Self::draw_tabs_section(frame, layout.tabs_section, state.clone());
        }

        Self::draw_command_overlay(frame, frame.area(), state);
    }

    fn draw_start_screen(frame: &mut Frame, area: Rect, state: Rc<TUIState>) {
        let border = Block::default()
            .title(Span::styled(
                " Hyperblow ",
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ))
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded);
        frame.render_widget(border, area);

        let inner = RectMath::inset(area, 4, 3);
        let pending_line = if state.has_pending_commands() {
            "Opening torrent..."
        } else {
            "No torrents loaded"
        };

        let content = Text::from(vec![
            Line::from(""),
            Line::styled("Hyperblow", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Line::from(""),
            Line::styled(pending_line, Style::default().fg(Color::Cyan)),
            Line::styled(
                format!("Downloads: {}", state.engine.download_directory().display()),
                Style::default().fg(Color::DarkGray),
            ),
            Line::from(""),
            Line::from("Press : and run file or magnet"),
            Line::styled(":file /path/to/file.torrent", Style::default().fg(Color::White)),
            Line::styled(":magnet magnet:?xt=...", Style::default().fg(Color::White)),
            Line::styled(":q", Style::default().fg(Color::White)),
        ]);

        frame.render_widget(
            Paragraph::new(content).alignment(Alignment::Center).wrap(Wrap { trim: true }),
            inner,
        );
    }

    fn draw_command_overlay(frame: &mut Frame, area: Rect, state: Rc<TUIState>) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let feedback = state.command_feedback();
        if !state.is_command_mode() && feedback.is_none() && !state.has_pending_commands() {
            return;
        }

        if state.is_command_mode() {
            CommandController::refresh_suggestions(state.as_ref());
        }

        let suggestions = if state.is_command_mode() {
            state.command_suggestions()
        } else {
            Vec::new()
        };
        let visible_suggestions = suggestions.len().min(6);
        let feedback_height = u16::from(feedback.is_some());
        let overlay_height = (3 + visible_suggestions as u16 + feedback_height).min(area.height.max(1));
        let overlay = Rect::new(
            area.x,
            area.y.saturating_add(area.height.saturating_sub(overlay_height)),
            area.width,
            overlay_height,
        );
        frame.render_widget(Clear, overlay);

        let block = Block::default()
            .title(" Command ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded);
        frame.render_widget(block, overlay);

        let inner = RectMath::inset(overlay, 1, 1);
        let mut lines = Vec::new();
        if state.is_command_mode() {
            lines.push(Line::from(vec![
                Span::styled(":", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(state.command_input()),
            ]));
        }

        if let Some(message) = feedback {
            let style = if state.command_feedback_is_error() {
                Style::default().fg(Color::Red)
            } else {
                Style::default().fg(Color::Green)
            };
            lines.push(Line::styled(message, style));
        }

        for (index, suggestion) in suggestions.into_iter().take(visible_suggestions).enumerate() {
            let selected = index == state.command_suggestion_index();
            let prefix = if selected { ">" } else { " " };
            let display_suggestion = CommandSuggester::display(&state.command_input(), &suggestion);
            let style = if selected {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            lines.push(Line::from(vec![
                Span::styled(prefix, style),
                Span::raw(" "),
                Span::styled(display_suggestion, style),
            ]));
        }

        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);

        if state.is_command_mode() && inner.width > 1 && inner.height > 0 {
            let input_width = state.command_input().chars().count() as u16;
            let max_cursor_x = inner.right().saturating_sub(1);
            let cursor_x = inner.x.saturating_add(1).saturating_add(input_width).min(max_cursor_x);
            frame.set_cursor_position(Position { x: cursor_x, y: inner.y });
        }
    }

    fn draw_tabs_section(frame: &mut Frame, area: Rect, state: Rc<TUIState>) {
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

        frame.render_widget(widget_tabs, chunks[0]);

        match *state.tab.borrow() {
            Tab::Details => DetailsTab::draw(frame, chunks[1], state.clone()),
            Tab::Bandwidth => BandwidthTab::draw(frame, chunks[1], state.clone()),
            Tab::Files => FilesTab::draw(frame, chunks[1], state.clone()),
            Tab::Trackers => TrackersTab::draw(frame, chunks[1], state.clone()),
            Tab::Peers => PeersTab::draw(frame, chunks[1], state.clone()),
            Tab::Pieces => PiecesTab::draw(frame, chunks[1], state.clone()),
        };
    }
}

struct MouseController;

impl MouseController {
    fn handle_event(mouse_event: event::MouseEvent, terminal_area: Rect, state: &TUIState) {
        state.mouse.set_x(mouse_event.column);
        state.mouse.set_y(mouse_event.row);

        let layout = AppLayout::new(terminal_area);
        let position = Position {
            x: mouse_event.column,
            y: mouse_event.row,
        };

        match mouse_event.kind {
            event::MouseEventKind::Down(event::MouseButton::Left) => {
                state.mouse.set_event(MouseEv::Clicked);
                if Self::select_tab_at(layout.tab_bar, position, state) {
                    return;
                }
                if Self::select_content_row_at(layout.tab_content_rows, position, state) {
                    return;
                }
                Self::select_torrent_at(layout.torrent_rows, position, state);
            }
            event::MouseEventKind::Drag(event::MouseButton::Left) => {
                state.mouse.set_event(MouseEv::Clicked);
                if !Self::select_content_row_at(layout.tab_content_rows, position, state) {
                    Self::select_torrent_at(layout.torrent_rows, position, state);
                }
            }
            event::MouseEventKind::Up(_) => {
                state.mouse.set_event(MouseEv::NotClicked);
            }
            event::MouseEventKind::ScrollUp => {
                if layout.torrents_section.contains(position) {
                    debug!("mouse scrolled torrents up");
                    state.decrement_torrent_index();
                } else if layout.tab_bar.contains(position) {
                    debug!("mouse scrolled tabs left");
                    state.decrement_tab_index();
                } else if HitTest::content_rows_area_for_current_tab(layout.tab_content_rows, state).contains(position) {
                    debug!("mouse scrolled content rows up");
                    state.decrement_content_row_index();
                }
            }
            event::MouseEventKind::ScrollDown => {
                if layout.torrents_section.contains(position) {
                    debug!("mouse scrolled torrents down");
                    state.increment_torrent_index();
                } else if layout.tab_bar.contains(position) {
                    debug!("mouse scrolled tabs right");
                    state.increment_tab_index();
                } else if HitTest::content_rows_area_for_current_tab(layout.tab_content_rows, state).contains(position) {
                    debug!("mouse scrolled content rows down");
                    state.increment_content_row_index();
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
        let Some(tab_index) = HitTest::tab_index_at(tab_bar, position) else {
            return false;
        };
        state.set_tab_index(tab_index);
        debug!(tab_index, "mouse selected tab");
        true
    }

    fn select_torrent_at(torrent_rows: Rect, position: Position, state: &TUIState) -> bool {
        let Some(torrent_index) = HitTest::torrent_index_at(torrent_rows, position) else {
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
        debug!(torrent_index, "mouse selected torrent");
        true
    }

    fn select_content_row_at(tab_content_rows: Rect, position: Position, state: &TUIState) -> bool {
        let rows_area = HitTest::content_rows_area_for_current_tab(tab_content_rows, state);
        let Some(row_index) = HitTest::content_row_index_at(rows_area, position) else {
            return false;
        };
        state.set_content_row_index(row_index);
        debug!(content_row_index = row_index, "mouse selected content row");
        true
    }
}

struct HitTest;

impl HitTest {
    fn content_rows_area_for_current_tab(tab_content_rows: Rect, state: &TUIState) -> Rect {
        match *state.tab.borrow() {
            Tab::Details | Tab::Pieces => RectMath::inset(tab_content_rows, 1, 1),
            Tab::Bandwidth | Tab::Files | Tab::Trackers | Tab::Peers => {
                let inner = RectMath::inset(tab_content_rows, 2, 2);
                Rect {
                    x: inner.x,
                    y: inner.y.saturating_add(2),
                    width: inner.width,
                    height: inner.height.saturating_sub(2),
                }
            }
        }
    }

    fn content_row_index_at(content_rows: Rect, position: Position) -> Option<usize> {
        if content_rows.contains(position) {
            Some(position.y.saturating_sub(content_rows.y) as usize)
        } else {
            None
        }
    }

    fn tab_index_at(tab_bar: Rect, position: Position) -> Option<usize> {
        let tab_content = RectMath::inset(tab_bar, 1, 1);
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
}

#[cfg(test)]
mod tests {
    use super::{
        AppLayout, AppRenderer, CommandController, HitTest, MouseController, RectMath, TAB_DIVIDER_WIDTH, TAB_PADDING_WIDTH, TAB_TITLES,
    };
    use crate::{
        engine::{Engine, TorrentSource},
        tui::tui_state::TUIState,
    };
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
    use ratatui::{
        backend::TestBackend,
        layout::{Position, Rect},
        Terminal,
    };
    use std::{
        fs,
        path::PathBuf,
        rc::Rc,
        sync::mpsc,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn renders_empty_app_without_panicking() {
        let backend = TestBackend::new(100, 32);
        let mut terminal = Terminal::new(backend).expect("test backend should initialize");
        let state = Rc::new(TUIState::new(Engine::new()));

        terminal
            .draw(|frame| AppRenderer::render(frame, state.clone()))
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
        assert!(rendered.contains(":file /path/to/file.torrent"));
        assert!(rendered.contains(":magnet magnet:?xt=..."));
        assert!(rendered.contains(":q"));
    }

    #[test]
    fn tab_key_applies_selected_command_suggestion() {
        let state = TUIState::new(Engine::new());
        let (sender, _receiver) = mpsc::channel();
        state.enter_command_mode();
        state.set_command_input("fi".to_string());
        CommandController::refresh_suggestions(&state);

        CommandController::handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::empty()), &state, sender);

        assert_eq!(state.command_input(), "file ");
    }

    #[test]
    fn command_q_requests_quit() {
        let state = TUIState::new(Engine::new());
        let (sender, _receiver) = mpsc::channel();
        state.enter_command_mode();
        state.set_command_input("q".to_string());

        let should_quit = CommandController::handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()), &state, sender);

        assert!(should_quit);
    }

    #[test]
    fn ctrl_c_cancels_command_mode_without_quitting() {
        let state = TUIState::new(Engine::new());
        let (sender, _receiver) = mpsc::channel();
        state.enter_command_mode();
        state.set_command_input("magnet magnet:?xt=urn:btih:".to_string());
        CommandController::refresh_suggestions(&state);

        let should_quit = CommandController::handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL), &state, sender);

        assert!(!should_quit);
        assert!(!state.is_command_mode());
        assert!(state.command_input().is_empty());
        assert!(state.command_suggestions().is_empty());
    }

    #[test]
    fn command_overlay_renders_path_hints() {
        let temp_dir = UiTestHarness::temp_dir();
        fs::write(temp_dir.join("alpha.torrent"), b"torrent").expect("test torrent should be created");

        let backend = TestBackend::new(100, 32);
        let mut terminal = Terminal::new(backend).expect("test backend should initialize");
        let state = Rc::new(TUIState::new(Engine::new()));
        state.enter_command_mode();
        state.set_command_input(format!("file {}", temp_dir.join("a").display()));

        terminal
            .draw(|frame| AppRenderer::render(frame, state.clone()))
            .expect("command overlay should render");

        let rendered = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(rendered.contains("Command"));
        assert!(rendered.contains("alpha.torrent"));

        fs::remove_dir_all(temp_dir).expect("temp dir should be removed");
    }

    #[test]
    fn finds_clicked_tab_from_rendered_tab_bar() {
        let tab_bar = Rect::new(0, 20, 100, 3);

        assert_eq!(HitTest::tab_index_at(tab_bar, UiTestHarness::tab_position(tab_bar, 0)), Some(0));
        assert_eq!(HitTest::tab_index_at(tab_bar, UiTestHarness::tab_position(tab_bar, 2)), Some(2));
        assert_eq!(HitTest::tab_index_at(tab_bar, Position { x: 99, y: 21 }), None);
    }

    #[test]
    fn maps_torrent_rows_to_visible_indexes() {
        let torrent_rows = Rect::new(2, 4, 80, 8);

        assert_eq!(HitTest::torrent_index_at(torrent_rows, Position { x: 3, y: 4 }), Some(0));
        assert_eq!(HitTest::torrent_index_at(torrent_rows, Position { x: 3, y: 7 }), Some(3));
        assert_eq!(HitTest::torrent_index_at(torrent_rows, Position { x: 3, y: 12 }), None);
    }

    #[test]
    fn maps_content_rows_to_visible_indexes() {
        let content_rows = Rect::new(2, 20, 80, 6);

        assert_eq!(HitTest::content_row_index_at(content_rows, Position { x: 3, y: 20 }), Some(0));
        assert_eq!(HitTest::content_row_index_at(content_rows, Position { x: 3, y: 23 }), Some(3));
        assert_eq!(HitTest::content_row_index_at(content_rows, Position { x: 3, y: 26 }), None);
    }

    #[test]
    fn mouse_click_selects_tab() {
        let terminal_area = Rect::new(0, 0, 100, 32);
        let layout = AppLayout::new(terminal_area);
        let state = TUIState::new(Engine::new());
        state.set_max_tab_index(TAB_TITLES.len() - 1);

        let position = UiTestHarness::tab_position(layout.tab_bar, 3);
        MouseController::handle_event(UiTestHarness::left_click(position), terminal_area, &state);

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
        let layout = AppLayout::new(terminal_area);
        let state = TUIState::new(engine);
        state.set_max_torrent_index(1);

        MouseController::handle_event(
            UiTestHarness::left_click(Position {
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
        let layout = AppLayout::new(terminal_area);
        let state = TUIState::new(Engine::new());
        state.set_max_tab_index(TAB_TITLES.len() - 1);
        state.set_tab_index(1);
        state.set_max_torrent_index(2);
        state.set_torrent_index(1);

        MouseController::handle_event(
            UiTestHarness::mouse_event(
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

        MouseController::handle_event(
            UiTestHarness::mouse_event(MouseEventKind::ScrollUp, UiTestHarness::tab_position(layout.tab_bar, 1)),
            terminal_area,
            &state,
        );
        assert_eq!(state.tab_index(), 0);
    }

    #[test]
    fn mouse_click_selects_active_tab_content_row() {
        let terminal_area = Rect::new(0, 0, 100, 32);
        let layout = AppLayout::new(terminal_area);
        let state = TUIState::new(Engine::new());
        state.set_max_content_row_index(4);

        let rows_area = HitTest::content_rows_area_for_current_tab(layout.tab_content_rows, &state);
        MouseController::handle_event(
            UiTestHarness::left_click(Position {
                x: rows_area.x + 1,
                y: rows_area.y + 2,
            }),
            terminal_area,
            &state,
        );

        assert_eq!(state.content_row_index(), 2);
    }

    #[test]
    fn mouse_wheel_over_tab_content_changes_content_row() {
        let terminal_area = Rect::new(0, 0, 100, 32);
        let layout = AppLayout::new(terminal_area);
        let state = TUIState::new(Engine::new());
        state.set_max_content_row_index(3);
        state.set_content_row_index(1);

        let rows_area = HitTest::content_rows_area_for_current_tab(layout.tab_content_rows, &state);
        MouseController::handle_event(
            UiTestHarness::mouse_event(
                MouseEventKind::ScrollDown,
                Position {
                    x: rows_area.x + 1,
                    y: rows_area.y,
                },
            ),
            terminal_area,
            &state,
        );
        assert_eq!(state.content_row_index(), 2);
    }

    struct UiTestHarness;

    impl UiTestHarness {
        fn tab_position(tab_bar: Rect, tab_index: usize) -> Position {
            let tab_content = RectMath::inset(tab_bar, 1, 1);
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
            Self::mouse_event(MouseEventKind::Down(MouseButton::Left), position)
        }

        fn mouse_event(kind: MouseEventKind, position: Position) -> MouseEvent {
            MouseEvent {
                kind,
                column: position.x,
                row: position.y,
                modifiers: KeyModifiers::empty(),
            }
        }

        fn temp_dir() -> PathBuf {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock should be after unix epoch")
                .as_nanos();
            let path = PathBuf::from("target")
                .join("ui-tests")
                .join(format!("t{}{}", std::process::id(), nonce % 1_000_000_000));
            fs::create_dir_all(&path).expect("temp dir should be created");
            path
        }
    }
}
