use super::mouse::Mouse;
//use super::sections::tabs_section::bandwidth_tab::TabSectionBandwidth;
//use super::sections::tabs_section::details_tab::TabSectionDetails;
//use super::sections::tabs_section::files_tab::TabSectionFiles;
use crate::engine::Engine;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};

#[derive(Default)]
pub enum Tab {
    #[default]
    Details,
    Bandwidth,
    Files,
    Trackers,
    Peers,
    Pieces,
}

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
        self.torrent_index.set(index.min(self.max_torrent_index.get()));
    }

    // Gets you the maximum tab index that can be achieved
    pub fn max_tab_index(&self) -> usize {
        self.max_tab_index.get()
    }

    // Sets the maximum tab index
    pub fn set_max_tab_index(&self, index: usize) {
        self.max_tab_index.set(index);
        if self.selected_tab_index.get() > index {
            self.set_tab_index(index);
        }
    }

    pub fn set_max_torrent_index(&self, index: usize) {
        self.max_torrent_index.set(index);
        if self.torrent_index.get() > index {
            self.torrent_index.set(index);
        }
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
    pub fn decrement_tab_index(&self) {
        let current_tab_index = self.tab_index();
        if current_tab_index == 0 {
            self.set_tab_index(self.max_tab_index());
        } else {
            self.set_tab_index(current_tab_index - 1);
        }
    }

    pub fn increment_torrent_index(&self) {
        let current_torrent_index = self.torrent_index();
        if current_torrent_index < self.max_torrent_index.get() {
            self.set_torrent_index(current_torrent_index + 1);
        }
    }

    pub fn decrement_torrent_index(&self) {
        let current_torrent_index = self.torrent_index();
        if current_torrent_index > 0 {
            self.set_torrent_index(current_torrent_index - 1);
        }
    }

    //    pub fn getTorrentsData() {}
    //

    /// Toggles either pause or resume of the torrent, which means that when this method is called
    /// with an index of torrent, it shall be paused or resumed
    pub fn toggle_torrent(&self, _index: usize) {}
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
        } else if self.tab_index() == 4 {
            *self.tab.borrow_mut() = Tab::Peers;
        } else if self.tab_index() == 5 {
            *self.tab.borrow_mut() = Tab::Pieces;
        } else {
            *self.tab.borrow_mut() = Tab::Details;
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
