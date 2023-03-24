use crate::tui::tui_state::TUIState;
use ratatui::{
    backend::Backend,
    layout::{Constraint, Layout, Rect},
    terminal::Frame,
    widgets::{Block, BorderType, Borders, Cell, Row, Table},
};
use std::rc::Rc;

/// Data for the Bandwidth Tab Section of TUI
pub struct BandwidthTab;

impl BandwidthTab {
    pub fn draw<B: Backend>(frame: &mut Frame<B>, area: Rect, state: Rc<TUIState>) {
        let area = Self::drawBorder(frame, area.clone());

        // Split the area for header row and torrents row
        let area: Vec<Rect> = Layout::default()
            .constraints([Constraint::Length(2), Constraint::Min(0)])
            .split(area)
            .iter()
            .cloned()
            .collect();

        //Self::draw_header_row(frame, area[0]);
        //Self::draw_tracker_rows(frame, area[1], state.clone());
    }

    // Given an area, it draws border around that area and then it simply returns a new area with a
    // a padding of 2
    fn drawBorder<B: Backend>(frame: &mut Frame<B>, area: Rect) -> Rect {
        // Builds the border around the given area
        let border_widget = Block::default()
            .border_type(BorderType::Thick)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded);

        frame.render_widget(border_widget, area.clone());

        // Recalculate the area inside, after border is built
        let area: Rect = Layout::default()
            .constraints([Constraint::Min(0)])
            .margin(2)
            .split(area)[0];

        area
    }
}
