use crate::tui::tui_state::TUIState;
use ratatui::{
    backend::Backend,
    layout::{Constraint, Layout, Rect},
    terminal::Frame,
    widgets::{Block, BorderType, Borders, Cell, Row, Table},
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
    pub fn draw<B: Backend>(frame: &mut Frame<B>, area: Rect, state: Rc<TUIState>) {
        // Create and render the border first
        let widget_border = Block::default()
            .border_type(BorderType::Thick)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded);

        frame.render_widget(widget_border, area.clone());

        // Recalculate the area after border is built
        let area: Rect = Layout::default()
            .constraints([Constraint::Min(0)])
            .margin(2)
            .split(area)[0];

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
    fn draw_header_row<B: Backend>(frame: &mut Frame<B>, area: Rect) {
        let table = Table::new([Row::new(vec![SN, URL, STATUS]), Row::new([""; 3])]).widths(&[
            Constraint::Percentage(SN_PERC),
            Constraint::Percentage(URL_PERC),
            Constraint::Percentage(STATUS_PERC),
        ]);
        frame.render_widget(table, area.to_owned());
    }

    // Draws all trackers informations that could be fit in the given area
    fn draw_tracker_rows<B: Backend>(frame: &mut Frame<B>, area: Rect, state: Rc<TUIState>) {
        let mut row_s = Vec::default();
        // Load the "torrent handle" from the currently selected torrent's index
        let current_torrent_index = state.torrent_index();
        let current_torrent_handle = { &(*state.engine.torrents.blocking_lock())[current_torrent_index] };

        // Go through all the trackers
        let trackers = current_torrent_handle.getTrackers();
        let ref trackers = *trackers.blocking_read();

        let mut sn = 1_u16;
        for tracker_s in trackers {
            for tracker in tracker_s {
                let widget_sn = Cell::from(sn.to_string());
                let widget_address = Cell::from(tracker.address.to_string());

                let row = Row::new([widget_sn, widget_address]);
                row_s.push(row);
                sn = sn + 1;
            }
        }

        let table = Table::new(row_s).widths(&[
            Constraint::Percentage(SN_PERC),
            Constraint::Percentage(URL_PERC),
            Constraint::Percentage(STATUS_PERC),
        ]);

        frame.render_widget(table, area.to_owned());
    }
}
