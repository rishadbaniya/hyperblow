#![allow(unused_imports)]

use crate::tui::tui_state::TUIState;
use ratatui::{
    backend::Backend,
    layout::{Constraint, Layout, Rect},
    terminal::Frame,
    widgets::{Block, BorderType, Borders, Cell, Row, Table},
};
use std::{cell::RefCell, rc::Rc, sync::Arc};
use tokio::sync::Mutex;

/// Data for the Bandwidth Tab Section of TUI
pub struct FilesTab;

impl FilesTab {
    pub fn draw<B: Backend,>(frame: &mut Frame<B,>, area: Rect, state: Rc<TUIState,>,) {
        // Create and render the border first
        let widget_border = Block::default()
            .border_type(BorderType::Thick,)
            .borders(Borders::ALL,)
            .border_type(BorderType::Rounded,);

        frame.render_widget(widget_border, area.clone(),);

        // Recalculate the area after border is built
        let area: Rect = Layout::default()
            .constraints([Constraint::Min(0,),],)
            .margin(2,)
            .split(area,)[0];

        // Split the area for header row and torrents row
        let area: Vec<Rect,> = Layout::default()
            .constraints([Constraint::Length(2,), Constraint::Min(0,),],)
            .split(area,)
            .iter()
            .cloned()
            .collect();
        //.into_iter()
        //.collect();

        //Self::draw_header_row(frame, area[0]);
        //Self::draw_tracker_rows(frame, area[1], state.clone());
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
