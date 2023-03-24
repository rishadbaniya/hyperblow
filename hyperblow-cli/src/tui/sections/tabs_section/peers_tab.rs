use crate::tui::tui_state::TUIState;
use ratatui::{
    backend::Backend,
    layout::{Constraint, Layout, Rect},
    terminal::Frame,
    widgets::{Block, BorderType, Borders},
};

use std::rc::Rc;

/// Data for the Bandwidth Tab Section of TUI
pub struct PeersTab;

impl PeersTab {
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

///// Data for the Details Tab Section of TUI
//pub struct TabSectionDetails {
///// Name of the torrent
//pub name: String,

///// Total downloaded in bytes
//pub bytes_completed: usize,

///// Total size in bytes
//pub bytes_total: usize,

///// Total no of pieces
//pub pieces_total: usize,

///// Downloaded no of pieces
//pub pieces_downloaded: usize,

///// Size of each piece
//pub piece_size: usize,

///// Total no of connected seeds
//pub connected_seeds: usize,

///// Total no of availaible seeds
//pub availaible_seeds: usize,

///// Total no of connected peers
//pub connected_peers: usize,

///// Total no of availaible peers
//pub availaible_peers: usize,

//pub download_speed: usize,

//pub upload_speed: usize,
//}

//impl TabSectionDetails {
//// TODO : Docs about this API
///// Given the frame and the area to render, it shall render the data for the
///// Details Tab
//pub fn renderWidget<B: Backend>(&self, frame: &mut Frame<B>, area: Rect) {
//let chunks = Layout::default()
//.direction(Direction::Vertical)
//.margin(2)
//.constraints(
//[
//Constraint::Length(2),
//Constraint::Percentage(15),
//Constraint::Length(2),
//Constraint::Length(2),
//Constraint::Percentage(15),
//Constraint::Min(0),
//]
//.as_ref(),
//)
//.split(area);
//let widget_border = Block::default().borders(Borders::ALL).border_type(BorderType::Rounded);
//let widget_torrent_name = Block::default().title(self.name.clone());
//let widget_bytes_completed_gauge = Gauge::default()
//.block(
//Block::default()
//.borders(Borders::ALL)
//.border_type(BorderType::Rounded)
//.title(self.amountCompleteInfo()),
//)
//.gauge_style(Style::default().fg(Color::White).bg(Color::Black).add_modifier(Modifier::ITALIC))
//.percent(self.getPercentageDownloaded());

//let widget_pieces_status = Block::default().title(self.getPiecesStatus());
//// The pieces info goes here for the sake of bytes value
////let widget_piece_info = Block::default().title(self.name.clone());

//frame.render_widget(widget_border, area);
//frame.render_widget(widget_torrent_name, chunks[0]);
//frame.render_widget(widget_bytes_completed_gauge, chunks[1]);
//frame.render_widget(widget_pieces_status, chunks[2]);
//}

//// TODO : Docs about this API
//fn getPercentageDownloaded(&self) -> u16 {
//let perc = if self.bytes_total != 0 {
//let perc = (self.bytes_completed as f32 / self.bytes_total as f32) * 100_f32;
//perc as u16
//} else {
//0_u16
//};
//perc
//}

//// TODO : Docs about this API
//fn amountCompleteInfo(&self) -> String {
//let delim = 1024_f32;

//let bytes_completed_kibibyte: f32 = self.bytes_completed as f32 / delim;
//let bytes_total_kibibyte: f32 = self.bytes_total as f32 / delim;

//if bytes_total_kibibyte < 1024_f32 {
//format!("{:.2} KiB Completed / {:.2} KiB Total", bytes_completed_kibibyte, bytes_total_kibibyte)
//} else {
//let bytes_completed_mibibyte: f32 = bytes_completed_kibibyte / delim;
//let bytes_total_mibibyte: f32 = bytes_total_kibibyte / delim;

//if bytes_total_mibibyte < 1024_f32 {
//format!("{:.2} MiB Completed / {:.2} MiB Total", bytes_completed_mibibyte, bytes_total_mibibyte)
//} else {
//let bytes_completed_gibibyte: f32 = bytes_completed_mibibyte / delim;
//let bytes_total_gibibyte: f32 = bytes_total_mibibyte / delim;

//format!("{:.2} GiB Completed / {:.2} GiB Total", bytes_completed_gibibyte, bytes_total_gibibyte)
//}
//}
//}

//// TODO : Docs about this API
//fn getPiecesStatus(&self) -> String {
//format!(
//"Pieces : {} Downloaded | {} Remaining | {} Total",
//self.pieces_downloaded,
//self.pieces_total - self.pieces_downloaded,
//self.pieces_total
//)
//}

//fn getDownloadAndUploadSpeed(&self) -> String {}
//}
