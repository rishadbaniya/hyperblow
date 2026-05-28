// There might be some debate on using Table, rather than customly creating each column with a
// certain size. One point to be noted is that we can't create wigets like Gauge inside of a Table
// widget, only text data can be rendered inside of Table widget

//use super::{mouse::MouseEv, tabs::bandwidth_tab::TabSectionBandwidth};
use crate::tui::tui_state::TUIState;
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    widgets::{Block, BorderType, Borders},
    Frame,
};
use std::rc::Rc;
pub mod bandwidth_tab;
pub mod details_tab;
pub mod files_tab;
pub mod peers_tab;
pub mod pieces_tab;
pub mod trackers_tab;

pub struct TabsSection;

impl TabsSection {
    pub fn draw(frame: &mut Frame, area: Rect, _state: Rc<TUIState>) {
        // Create and render the border first
        let border = Block::default()
            .border_type(BorderType::Thick)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            //.title(Span::styled(" Hyperblow ", Style::default().fg(Color::Yellow)))
            .title_alignment(Alignment::Center);
        frame.render_widget(border, area);

        // Recalculate the area after border is built
        let area = Layout::default().constraints([Constraint::Min(0)]).margin(2).split(area)[0];

        // Split the area for header row and torrents row
        let _area = Layout::default()
            .constraints([Constraint::Length(2), Constraint::Min(0)])
            .split(area);

        // Draw the header and torrents column
    }
}
