use crate::tui::tui_state::TUIState;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
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
        let torrent_handles = state.engine.torrents.blocking_lock();
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

        let mut sn = 1_u16;
        for tracker in current_torrent_handle.tracker_snapshots() {
            let sn_widget = Cell::from(sn.to_string());
            let url_widget = Cell::from(tracker.url);
            let tracker_state_color = if tracker.is_error { Color::Red } else { Color::Green };
            let tracker_state_widget = Cell::from(tracker.status).style(Style::default().fg(tracker_state_color));
            let row = Row::new([sn_widget, url_widget, tracker_state_widget]);
            row_s.push(row);
            sn += 1;
        }

        if row_s.is_empty() {
            row_s.push(Row::new(["", "No trackers available", ""]));
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
