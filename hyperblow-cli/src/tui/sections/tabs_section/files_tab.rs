#![allow(unused_imports)]

use crate::tui::tui_state::TUIState;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    widgets::{Block, BorderType, Borders, Paragraph, Row, Table},
    Frame,
};
use std::rc::Rc;

/// Data for the Bandwidth Tab Section of TUI
pub struct FilesTab;

impl FilesTab {
    pub fn draw(frame: &mut Frame, area: Rect, state: Rc<TUIState>) {
        let area = Self::drawBorder(frame, area);

        // Split the area for header row and torrents row
        let area: Vec<Rect> = Layout::default()
            .constraints([Constraint::Length(2), Constraint::Min(0)])
            .split(area)
            .iter()
            .cloned()
            .collect();

        let table = Table::new([Row::new(["Path"])], [Constraint::Percentage(100)]);
        frame.render_widget(table, area[0]);

        let torrent_handles = state.engine.torrents.blocking_lock();
        let Some(handle) = torrent_handles.get(state.torrent_index()) else {
            frame.render_widget(Paragraph::new("No torrent selected"), area[1]);
            return;
        };

        let names = handle.file_tree_names();
        if names.is_empty() {
            frame.render_widget(Paragraph::new("No file tree available yet"), area[1]);
            return;
        }

        let rows = names.into_iter().map(|name| Row::new([name]));
        let table = Table::new(rows, [Constraint::Percentage(100)]);
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
    //pub fn renderWidget<B: Backend>(&self, frame: &mut Frame<B>, area: Rect) {
    //let chunks = Layout::default()
    //.constraints([
    //Constraint::Length(1),
    //Constraint::Length(1),
    //Constraint::Length(1),
    //Constraint::Length(1),
    //Constraint::Length(1),
    //Constraint::Length(1),
    //Constraint::Length(1),
    //Constraint::Length(1),
    //Constraint::Length(1),
    //Constraint::Length(1),
    //Constraint::Length(1),
    //Constraint::Length(1),
    //Constraint::Length(1),
    //Constraint::Length(1),
    //Constraint::Length(1),
    //Constraint::Length(1),
    //Constraint::Length(1),
    //Constraint::Length(1),
    //Constraint::Length(1),
    //Constraint::Length(1),
    //Constraint::Length(1),
    //Constraint::Length(1),
    //Constraint::Length(1),
    //Constraint::Length(1),
    //Constraint::Length(1),
    //Constraint::Length(1),
    //Constraint::Length(1),
    //Constraint::Length(1),
    //Constraint::Length(1),
    //Constraint::Length(1),
    //Constraint::Length(1),
    //Constraint::Length(1),
    //])
    //.direction(Direction::Vertical)
    //.split(area);

    //for (ind, b) in self.widgets.borrow().iter().enumerate() {
    //if ind < 30 {
    //frame.render_widget(b.clone(), chunks[ind]);
    //} else {
    //break;
    //}
    //}
    //}
    //pub async fn loadWidgets(&self) {
    //let x = self.file_tree.lock().await.tabs_traverse_names(0).await;
    //for i in x {
    //self.widgets.borrow_mut().push(Block::default().title(i));
    //}
    //}
}
