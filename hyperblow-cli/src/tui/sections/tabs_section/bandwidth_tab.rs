use crate::{tui::tui_state::TUIState, utils};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    widgets::{Block, BorderType, Borders, Paragraph, Row, Table},
    Frame,
};
use std::rc::Rc;

/// Data for the Bandwidth Tab Section of TUI
pub struct BandwidthTab;

impl BandwidthTab {
    pub fn draw(frame: &mut Frame, area: Rect, state: Rc<TUIState>) {
        let area = Self::drawBorder(frame, area);

        // Split the area for header row and torrents row
        let area: Vec<Rect> = Layout::default()
            .constraints([Constraint::Length(2), Constraint::Min(0)])
            .split(area)
            .iter()
            .cloned()
            .collect();

        let table = Table::new([Row::new(["Metric", "Value"])], [Constraint::Length(18), Constraint::Min(10)]);
        frame.render_widget(table, area[0]);

        let torrent_handles = state.engine.torrents.blocking_lock();
        let Some(handle) = torrent_handles.get(state.torrent_index()) else {
            frame.render_widget(Paragraph::new("No torrent selected"), area[1]);
            return;
        };

        let rows = [
            Row::new([
                "Download".to_string(),
                format!("{}/s", utils::bytes_to_human_readable(handle.download_speed())),
            ]),
            Row::new([
                "Upload".to_string(),
                format!("{}/s", utils::bytes_to_human_readable(handle.upload_speed())),
            ]),
        ];
        let table = Table::new(rows, [Constraint::Length(18), Constraint::Min(10)]);
        frame.render_widget(table, area[1]);
    }

    // Given an area, it draws border around that area and then it simply returns a new area with a
    // a padding of 2
    fn drawBorder(frame: &mut Frame, area: Rect) -> Rect {
        // Builds the border around the given area
        let border_widget = Block::default()
            .border_type(BorderType::Thick)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded);

        frame.render_widget(border_widget, area);

        // Recalculate the area inside, after border is built
        let area: Rect = Layout::default().constraints([Constraint::Min(0)]).margin(2).split(area)[0];

        area
    }
}
