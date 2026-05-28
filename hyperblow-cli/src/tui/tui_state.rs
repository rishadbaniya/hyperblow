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

    content_row_index: Cell<usize>,

    max_content_row_index: Cell<usize>,

    command_mode: Cell<bool>,

    command_input: RefCell<String>,

    command_suggestions: RefCell<Vec<String>>,

    command_suggestion_index: Cell<usize>,

    command_feedback: RefCell<Option<String>>,

    command_feedback_is_error: Cell<bool>,

    pending_command_count: Cell<usize>,
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
        let content_row_index = Cell::new(0);
        let max_content_row_index = Cell::new(0);
        let command_mode = Cell::new(false);
        let command_input = RefCell::new(String::new());
        let command_suggestions = RefCell::new(Vec::new());
        let command_suggestion_index = Cell::new(0);
        let command_feedback = RefCell::new(None);
        let command_feedback_is_error = Cell::new(false);
        let pending_command_count = Cell::new(0);

        TUIState {
            max_tab_index,
            selected_tab_index,
            engine,
            mouse,
            tab,
            torrent_index,
            max_torrent_index,
            content_row_index,
            max_content_row_index,
            command_mode,
            command_input,
            command_suggestions,
            command_suggestion_index,
            command_feedback,
            command_feedback_is_error,
            pending_command_count,
        }
    }

    // Gets you the current tab index
    pub fn tab_index(&self) -> usize {
        self.selected_tab_index.get()
    }

    // Sets the current tab index
    pub fn set_tab_index(&self, index: usize) {
        self.selected_tab_index.set(index);
        self.content_row_index.set(0);
        self.max_content_row_index.set(0);
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

    pub fn content_row_index(&self) -> usize {
        self.content_row_index.get()
    }

    pub fn set_content_row_index(&self, index: usize) {
        self.content_row_index.set(index.min(self.max_content_row_index.get()));
    }

    pub fn set_max_content_row_index(&self, index: usize) {
        self.max_content_row_index.set(index);
        if self.content_row_index.get() > index {
            self.content_row_index.set(index);
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

    pub fn increment_content_row_index(&self) {
        let current_content_row_index = self.content_row_index();
        if current_content_row_index < self.max_content_row_index.get() {
            self.set_content_row_index(current_content_row_index + 1);
        }
    }

    pub fn decrement_content_row_index(&self) {
        let current_content_row_index = self.content_row_index();
        if current_content_row_index > 0 {
            self.set_content_row_index(current_content_row_index - 1);
        }
    }

    pub fn is_command_mode(&self) -> bool {
        self.command_mode.get()
    }

    pub fn enter_command_mode(&self) {
        self.command_mode.set(true);
        self.clear_command_feedback();
    }

    pub fn exit_command_mode(&self) {
        self.command_mode.set(false);
        self.command_suggestion_index.set(0);
    }

    pub fn command_input(&self) -> String {
        self.command_input.borrow().clone()
    }

    pub fn set_command_input(&self, input: String) {
        *self.command_input.borrow_mut() = input;
        self.command_suggestion_index.set(0);
    }

    pub fn push_command_char(&self, character: char) {
        self.command_input.borrow_mut().push(character);
        self.command_suggestion_index.set(0);
        self.clear_command_feedback();
    }

    pub fn pop_command_char(&self) {
        self.command_input.borrow_mut().pop();
        self.command_suggestion_index.set(0);
        self.clear_command_feedback();
    }

    pub fn clear_command_input(&self) {
        self.command_input.borrow_mut().clear();
        self.command_suggestion_index.set(0);
    }

    pub fn set_command_suggestions(&self, suggestions: Vec<String>) {
        let max_index = suggestions.len().saturating_sub(1);
        *self.command_suggestions.borrow_mut() = suggestions;
        if self.command_suggestion_index.get() > max_index {
            self.command_suggestion_index.set(max_index);
        }
    }

    pub fn command_suggestions(&self) -> Vec<String> {
        self.command_suggestions.borrow().clone()
    }

    pub fn command_suggestion_index(&self) -> usize {
        self.command_suggestion_index.get()
    }

    pub fn increment_command_suggestion_index(&self) {
        let suggestions_len = self.command_suggestions.borrow().len();
        if suggestions_len == 0 {
            self.command_suggestion_index.set(0);
            return;
        }

        let next_index = (self.command_suggestion_index.get() + 1) % suggestions_len;
        self.command_suggestion_index.set(next_index);
    }

    pub fn decrement_command_suggestion_index(&self) {
        let suggestions_len = self.command_suggestions.borrow().len();
        if suggestions_len == 0 {
            self.command_suggestion_index.set(0);
            return;
        }

        let next_index = if self.command_suggestion_index.get() == 0 {
            suggestions_len - 1
        } else {
            self.command_suggestion_index.get() - 1
        };
        self.command_suggestion_index.set(next_index);
    }

    pub fn selected_command_suggestion(&self) -> Option<String> {
        self.command_suggestions.borrow().get(self.command_suggestion_index.get()).cloned()
    }

    pub fn set_command_feedback(&self, message: String, is_error: bool) {
        *self.command_feedback.borrow_mut() = Some(message);
        self.command_feedback_is_error.set(is_error);
    }

    pub fn command_feedback(&self) -> Option<String> {
        self.command_feedback.borrow().clone()
    }

    pub fn command_feedback_is_error(&self) -> bool {
        self.command_feedback_is_error.get()
    }

    pub fn clear_command_feedback(&self) {
        *self.command_feedback.borrow_mut() = None;
        self.command_feedback_is_error.set(false);
    }

    pub fn increment_pending_commands(&self) {
        self.pending_command_count.set(self.pending_command_count.get().saturating_add(1));
    }

    pub fn decrement_pending_commands(&self) {
        self.pending_command_count.set(self.pending_command_count.get().saturating_sub(1));
    }

    pub fn has_pending_commands(&self) -> bool {
        self.pending_command_count.get() > 0
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
