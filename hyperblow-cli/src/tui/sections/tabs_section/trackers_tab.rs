use crate::tui::tui_state::TUIState;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, Cell, Row, Table},
    Frame,
};
use std::rc::Rc;

const SN: &str = "SN";
const SN_PERC: u16 = 5;

const URL: &str = "URL";
const URL_PERC: u16 = 35;

const STATUS: &str = "Status";
const STATUS_PERC: u16 = 60;

pub struct TrackersTab {}

impl TrackersTab {
    /// Draws all the trackers informations on the given area from the given TUIState in the given
    /// area
    pub fn draw(frame: &mut Frame, area: Rect, state: Rc<TUIState>) {
        // Create and render the border first
        let widget_border = Block::default()
            .border_type(BorderType::Thick)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded);

        frame.render_widget(widget_border, area);

        // Recalculate the area after border is built
        let area: Rect = Layout::default().constraints([Constraint::Min(0)]).margin(2).split(area)[0];

        // Split the area for header row and torrents row
        let area: Vec<Rect> = Layout::default()
            .constraints([Constraint::Length(2), Constraint::Min(0)])
            .split(area)
            .iter()
            .cloned()
            .collect();
        //.into_iter()
        //.collect();

        Self::draw_header_row(frame, area[0]);
        Self::draw_tracker_rows(frame, area[1], state.clone());
    }

    // Draws header row and leaves one row spacing below
    fn draw_header_row(frame: &mut Frame, area: Rect) {
        let table = Table::new(
            [Row::new(vec![SN, URL, STATUS]), Row::new([""; 3])],
            [
                Constraint::Percentage(SN_PERC),
                Constraint::Percentage(URL_PERC),
                Constraint::Percentage(STATUS_PERC),
            ],
        );
        frame.render_widget(table, area.to_owned());
    }

    // Draws all trackers informations that could be fit in the given area
    fn draw_tracker_rows(frame: &mut Frame, area: Rect, state: Rc<TUIState>) {
        let mut row_s = Vec::default();

        let current_torrent_index = state.torrent_index();
        let Some(torrent_handles) = state.engine.torrent_snapshot() else {
            let table = Table::new(
                [Row::new(["", "Torrent state is updating...", ""])],
                [
                    Constraint::Percentage(SN_PERC),
                    Constraint::Percentage(URL_PERC),
                    Constraint::Percentage(STATUS_PERC),
                ],
            );
            frame.render_widget(table, area);
            return;
        };
        let Some(current_torrent_handle) = torrent_handles.get(current_torrent_index) else {
            let table = Table::new(
                [Row::new(["", "No torrent selected", ""])],
                [
                    Constraint::Percentage(SN_PERC),
                    Constraint::Percentage(URL_PERC),
                    Constraint::Percentage(STATUS_PERC),
                ],
            );
            frame.render_widget(table, area);
            return;
        };

        let trackers = current_torrent_handle.tracker_snapshots();
        if trackers.is_empty() {
            state.set_max_content_row_index(0);
            row_s.push(Row::new(["", "No trackers available", ""]));
        }

        let visible_rows = area.height as usize;
        let max_selected_index = trackers.len().saturating_sub(1);
        state.set_max_content_row_index(max_selected_index);
        let selected_index = state.content_row_index().min(max_selected_index);
        let viewport_start = TrackerViewport::start_for_selection(selected_index, trackers.len(), visible_rows);

        for (index, tracker) in trackers.into_iter().enumerate().skip(viewport_start).take(visible_rows) {
            let sn_widget = Cell::from((index + 1).to_string());
            let url_widget = Cell::from(tracker.url);
            let tracker_state_color = if tracker.is_error { Color::Red } else { Color::Green };
            let tracker_state_widget = Cell::from(tracker.status).style(Style::default().fg(tracker_state_color));
            let row = Row::new([sn_widget, url_widget, tracker_state_widget]);
            let row = if index == selected_index {
                row.style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
            } else {
                row
            };
            row_s.push(row);
        }

        let table = Table::new(
            row_s,
            [
                Constraint::Percentage(SN_PERC),
                Constraint::Percentage(URL_PERC),
                Constraint::Percentage(STATUS_PERC),
            ],
        );

        frame.render_widget(table, area.to_owned());
    }
}

struct TrackerViewport;

impl TrackerViewport {
    fn start_for_selection(selected_index: usize, total_rows: usize, visible_rows: usize) -> usize {
        if total_rows == 0 || visible_rows == 0 {
            return 0;
        }

        let max_start = total_rows.saturating_sub(visible_rows);
        selected_index.saturating_add(1).saturating_sub(visible_rows).min(max_start)
    }
}

#[cfg(test)]
mod tests {
    use super::{TrackerViewport, TrackersTab};
    use crate::{
        engine::{Engine, TorrentSource},
        tui::tui_state::TUIState,
    };
    use ratatui::{backend::TestBackend, layout::Rect, Terminal};
    use std::rc::Rc;

    #[test]
    fn tracker_rows_scroll_with_content_offset() {
        let engine = Engine::new();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("test runtime should initialize");
        runtime.block_on(async {
            engine
                .spawn(TorrentSource::MagnetURI(MagnetTestFixture::many_trackers_uri(13)))
                .await
                .expect("magnet should spawn");
        });

        let state = Rc::new(TUIState::new(engine));
        let backend = TestBackend::new(100, 12);
        let mut terminal = Terminal::new(backend).expect("test backend should initialize");

        terminal
            .draw(|frame| TrackersTab::draw(frame, Rect::new(0, 0, 100, 12), state.clone()))
            .expect("trackers should render");
        for _ in 0..12 {
            state.increment_content_row_index();
        }
        assert_eq!(state.content_row_index(), 12);
        terminal
            .draw(|frame| TrackersTab::draw(frame, Rect::new(0, 0, 100, 12), state.clone()))
            .expect("scrolled trackers should render");

        let rendered = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();

        assert!(rendered.contains("tracker12.example.com"));
    }

    #[test]
    fn tracker_viewport_keeps_last_item_reachable() {
        assert_eq!(TrackerViewport::start_for_selection(12, 13, 6), 7);
    }

    struct MagnetTestFixture;

    impl MagnetTestFixture {
        fn many_trackers_uri(count: usize) -> String {
            let mut uri = "magnet:?xt=urn:btih:08ada5a7a6183aae1e09d831df6748d566095a10&dn=ManyTrackers".to_string();
            for index in 0..count {
                uri.push_str(&format!("&tr=udp://tracker{index}.example.com:6969"));
            }
            uri
        }
    }
}
