use super::mouse::Mouse;
//use super::sections::tabs_section::bandwidth_tab::TabSectionBandwidth;
//use super::sections::tabs_section::details_tab::TabSectionDetails;
//use super::sections::tabs_section::files_tab::TabSectionFiles;
use crate::engine::Engine;
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    terminal::Frame,
    text::Text,
    widgets,
    widgets::{Block, BorderType, Borders, Gauge},
};
use std::{
    cell::{Cell, RefCell},
    fmt::format,
    rc::Rc,
    sync::Arc,
};

pub enum Tab {
    Details,
    Bandwidth,
    Files,
    Trackers,
    Peers,
    Pieces,
    None,
}

impl Default for Tab {
    fn default() -> Self {
        Self::None
    }
}

/// Data for the Details Tab Section of TUI
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
//let widget_download_upload_speed = Block::default().title(self.getDownloadAndUploadSpeed());
//let widget_bytes_completed_gauge = Gauge::default()
//.block(
//Block::default()
//.borders(Borders::ALL)
//.border_type(BorderType::Rounded)
//.title(self.getAmountCompleteInfo()),
//)
//.gauge_style(Style::default().fg(Color::White).bg(Color::Black).add_modifier(Modifier::ITALIC))
//.percent(self.getPercentageDownloaded());

//let widget_pieces_status = Block::default().title(self.getPiecesStatus());
//let widget_piece_size = Block::default().title(self.getPiecesSize());
//// The pieces info goes here for the sake of bytes value
////let widget_piece_info = Block::default().title(self.name.clone());

//frame.render_widget(widget_border, area);
//frame.render_widget(widget_torrent_name, chunks[0]);
//frame.render_widget(widget_bytes_completed_gauge, chunks[1]);
//frame.render_widget(widget_download_upload_speed, chunks[2]);
//frame.render_widget(widget_pieces_status, chunks[3]);
//frame.render_widget(widget_piece_size, chunks[4]);
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
//fn getAmountCompleteInfo(&self) -> String {
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

//// TODO : Docs about this API
//fn getDownloadAndUploadSpeed(&self) -> String {
//let speed = |s: usize| -> String {
//let kibibytes = s as f32 / 1024_f32;
//if kibibytes < 1024_f32 {
//format!("{:.2} KiB/s", kibibytes)
//} else {
//let mibibytes = kibibytes / 1024_f32;
//if mibibytes < 1024_f32 {
//format!("{:.2} MiB/s", mibibytes)
//} else {
//let gibibytes = mibibytes / 1024_f32;
//format!("{:.2} GiB/s", gibibytes)
//}
//}
//};

//format!("Download Speed : {} | Upload Speed {}", speed(424234), speed(2342))
//}

//// TODO : Docs about this API
//fn getPiecesSize(&self) -> String {
//let size = |s: usize| -> String {
//let kibibytes = s as f32 / 1024_f32;
//if kibibytes < 1024_f32 {
//format!("{:.2} KiB", kibibytes)
//} else {
//let mibibytes = kibibytes / 1024_f32;
//format!("{:.2} MiB", mibibytes)
//}
//};

//format!("Piece Size : {}", size(self.piece_size))
//}
//}

//pub struct TabSectionTrackers {}

//pub struct TabSectionPeers {}

//pub struct TabSectionPieces {}

/// Holds and upates the state for TUI, works on interior mutability
/// One doesn't need any &mut TUIState to change its state and peek its state
///
/// One can simply use Rc<TUIState> Or Arc<TUIState>, update and get the state
pub struct TUIState {
    /// The maximum tab index in TabSection
    max_tab_index: Cell<usize>,

    /// The selected tab index in TabSection
    selected_tab_index: Cell<usize>,

    /// An Arc Pointer to Engine, on which we can operate without using its mutable reference as
    /// it works on interior mutability
    pub engine: Arc<Engine>,

    /// An Rc Pointer to Mouse State, it be modified to hold different Mouse data
    pub mouse: Rc<Mouse>,

    pub tab: Rc<RefCell<Tab>>,

    torrent_index: Cell<usize>,

    max_torrent_index: Cell<usize>,
}

impl TUIState {
    /// Creates a new TUIState
    pub fn new(engine: Arc<Engine>) -> Self {
        // Initially we set the max tab index to 0, inside the drawTabsSection it will be
        // set to the max index automatically, the only time max_tab_index's value is extracted is
        // in the crosstern::event::poll function, which is called after the drawTabsSection
        let max_tab_index = Cell::new(0);

        // Default selected tab index of Tabs Section
        let selected_tab_index = Cell::new(0);

        // Mouse details and events for TUI
        let mouse = Rc::new(Mouse::default());

        let tab = Rc::default();

        let torrent_index = Cell::new(0);

        let max_torrent_index = Cell::new(0);

        TUIState {
            max_tab_index,
            selected_tab_index,
            engine,
            mouse,
            tab,
            torrent_index,
            max_torrent_index,
        }
    }

    // Gets you the current tab index
    pub fn tab_index(&self) -> usize {
        self.selected_tab_index.get()
    }

    // Sets the current tab index
    pub fn set_tab_index(&self, index: usize) {
        self.selected_tab_index.set(index);
        self.loadTab();
    }

    pub fn torrent_index(&self) -> usize {
        self.torrent_index.get()
    }

    pub fn set_torrent_index(&self, index: usize) {
        self.torrent_index.set(index);
    }

    // Gets you the maximum tab index that can be achieved
    pub fn max_tab_index(&self) -> usize {
        self.max_tab_index.get()
    }

    // Sets the maximum tab index
    pub fn set_max_tab_index(&self, index: usize) {
        self.max_tab_index.set(index);
    }

    // Increments the tab index by 1
    pub fn increment_tab_index(&self) {
        let current_tab_index = self.tab_index();
        if current_tab_index == self.max_tab_index() {
            self.set_tab_index(0);
        } else {
            self.set_tab_index(current_tab_index + 1);
        }
    }

    // TODO : Make use of crossterm key combination for Decrementing the tab index
    // Decrements the tab index by 1
    pub fn decrement_tab_index(&self) {}

    //    pub fn getTorrentsData() {}
    //

    /// Toggles either pause or resume of the torrent, which means that when this method is called
    /// with an index of torrent, it shall be paused or resumed
    pub fn toggle_torrent(&self, index: usize) {}
    // Gets the data to be displayed on the TorrentsSection
    // It has following structure of HashMap represented in JSON Structure:
    // {
    //     "Name" : "XYZ",
    //     "Speed In" : "2000000" // Data is represented in bytes/s
    //     "Speed In" : "1000000" // Data is represented in bytes/s
    //     "Progress" : "24242432/23423423" // Bytes Completed Out Of Total Bytes
    // }
    //

    pub fn loadTab(&self) {
        if self.tab_index() == 0 {
            *self.tab.borrow_mut() = Tab::Details;
        } else if self.tab_index() == 1 {
            *self.tab.borrow_mut() = Tab::Bandwidth;
        } else if self.tab_index() == 2 {
            *self.tab.borrow_mut() = Tab::Files;
        } else if self.tab_index() == 3 {
            *self.tab.borrow_mut() = Tab::Trackers;
        } else {
            *self.tab.borrow_mut() = Tab::None;
        }
    }

    //pub async fn refresh(&self) {
    //self.loadTabSection(0).await;
    //}

    //async fn getTabSectionDetails(&self, index: usize) -> TabSectionDetails {
    //let torrent_handle = self.engine.torrents.lock().await[index].clone();

    //let name = torrent_handle.name();
    //let bytes_completed = torrent_handle.bytes_complete();
    //let bytes_total = torrent_handle.bytes_total();
    //let pieces_total = torrent_handle.pieces_total();
    //let pieces_downloaded = torrent_handle.pieces_downloaded();
    //let piece_size = torrent_handle.piece_size();
    //let connected_seeds = 0;
    //let availaible_seeds = 0;
    //let connected_peers = 0;
    //let availaible_peers = 0;
    //let download_speed = 0;
    //let upload_speed = 0;

    //TabSectionDetails {
    //name,
    //bytes_completed,
    //bytes_total,
    //pieces_total,
    //pieces_downloaded,
    //piece_size,
    //connected_seeds,
    //availaible_seeds,
    //connected_peers,
    //availaible_peers,
    //download_speed,
    //upload_speed,
    //}
    //}

    //async fn getTabSectionBandwidth(&self, index: usize) -> TabSectionBandwidth {
    //TabSectionBandwidth {
    //download_speed: 0,
    //upload_speed: 0,
    //}
    //}

    //async fn getTabSectionFiles(&self, index: usize) -> TabSectionFiles {
    //let torrent_handle = self.engine.torrents.lock().await[index].clone();

    //let file_tree = torrent_handle.getFileTree();
    //let widgets = Rc::default();
    //let mut x = TabSectionFiles { file_tree, widgets };
    //x.loadWidgets().await;
    //x
    //}
}
