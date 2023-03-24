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
    pub fn draw<B: Backend,>(frame: &mut Frame<B,>, area: Rect, state: Rc<TUIState,>,) {
        //// Create and render the border first
        //let widget_border = Block::default()
        //.border_type(BorderType::Thick)
        //.borders(Borders::ALL)
        //.border_type(BorderType::Rounded);

        //frame.render_widget(widget_border, area.clone());

        //// Recalculate the area after border is built
        //let area: Rect = Layout::default().constraints([Constraint::Min(0)]).margin(2).split(area)[0];

        //// Split the area for header row and torrents row
        //let area: Vec<Rect> = Layout::default()
        //.constraints([Constraint::Length(2), Constraint::Min(0)])
        //.split(area)
        //.iter()
        //.cloned()
        //.collect();
        //.into_iter()
        //.collect();

        //Self::draw_header_row(frame, area[0]);
        //Self::draw_tracker_rows(frame, area[1], state.clone());
    }
    /*pub fn renderWidget<B: Backend>(&self, frame: &mut Frame<B>, area: Rect) {*/
    /*let datasets = vec![*/
    /*Dataset::default()*/
    /*.name("Upload Speed : KiB/s")*/
    /*.marker(Marker::Dot)*/
    /*.graph_type(GraphType::Line)*/
    /*.style(Style::default().fg(Color::Cyan))*/
    /*.data(&[(0.0, 5.0), (1.0, 6.0), (1.5, 6.434)]),*/
    /*Dataset::default()*/
    /*.name("Download Speed : MiB/s")*/
    /*.marker(Marker::Dot)*/
    /*.graph_type(GraphType::Line)*/
    /*.style(Style::default().fg(Color::Magenta))*/
    /*.data(&[(4.0, 5.0), (5.0, 8.0), (7.66, 13.5)]),*/
    /*];*/

    /*let widget_download_bandwidth_chart = Chart::new(datasets)*/
    /*.x_axis(*/
    /*Axis::default()*/
    /*.title(Span::styled("Time", Style::default().fg(Color::Red)))*/
    /*.style(Style::default().fg(Color::White))*/
    /*.bounds([0.0, 10.0])*/
    /*.labels(["0.0", "5.0", "10.0"].iter().cloned().map(Span::from).collect()),*/
    /*)*/
    /*.y_axis(*/
    /*Axis::default()*/
    /*.title(Span::styled("Bandwidth", Style::default().fg(Color::Red)))*/
    /*.style(Style::default().fg(Color::White))*/
    /*.bounds([0.0, 10.0])*/
    /*.labels(["0.0", "5.0", "10.0"].iter().cloned().map(Span::from).collect()),*/
    /*);*/

    /*frame.render_widget(widget_download_bandwidth_chart, area);*/
    /*}*/
}
