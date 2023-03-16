//TODO : Add a way to toggle the state of the Download

use crate::work::file::{File, FileType};
use std::cell;
use std::sync::Arc;
use tokio::sync::{Mutex, MutexGuard};
use tui::text;
use tui::widgets::{Block, Borders};
use tui::{
    backend::Backend,
    layout::Alignment,
    layout::{Constraint, Rect},
    style::Color,
    style::Style,
    widgets::{BorderType, Cell, Row, Table},
    Frame,
};

// Name : Name of the root file or the folder
// Type : Type of the file, if it's directory of a regular file
// Download : Should download or not (Yes/No)
// Progress : Progress [X Mb / Y Mb | 10.5%]
const NAME: &str = "Name";
const NAME_WIDTH_PERCENTAGE: u16 = 60;

const TYPE: &str = "Type";
const TYPE_WIDTH_PERCENTAGE: u16 = 8;

const DOWNLOAD: &str = "Download";
const DOWNLOAD_WIDTH_PERCENTAGE: u16 = 8;

const PROGRESS: &str = "Progress";
const PROGRESS_WIDTH_PERCENTAGE: u16 = 24;

// Stores all the necessary states required to render Files Tab
pub struct FilesState {
    // Start and end index of the range of items that's being drawn
    // Note : Used in scroll to draw the range of data to be drawn
    top_index: cell::Cell<u16>,
    bottom_index: cell::Cell<u16>,
    // Scroll state
    // if current > previous then we can say user wanted to scroll down
    // if previous < current then we can say user wanted to scroll up
    // The difference is just 1 or -1
    scroll_state_current: cell::Cell<i16>,
    scroll_state_previous: cell::Cell<i16>,
    pub rect: Rect,
    pub file: Arc<Mutex<File>>,
    // Name of the torrent
    pub name: String,
}

impl FilesState {
    pub fn new() -> Self {
        FilesState {
            rect: Rect::new(0, 0, 0, 0),
            top_index: cell::Cell::new(0),
            bottom_index: cell::Cell::new(0),
            scroll_state_current: cell::Cell::new(0),
            scroll_state_previous: cell::Cell::new(0),
            file: Arc::new(Mutex::new(File {
                name: String::from("root"),
                file_type: FileType::DIRECTORY,
                inner_files: Some(Vec::new()),
                size: 0,
                should_download: true,
            })),
            name: String::from(""),
        }
    }

    pub fn set_top_index(&self, v: u16) {
        self.top_index.set(v);
    }

    pub fn get_top_index(&self) -> u16 {
        self.top_index.get()
    }

    pub fn set_bottom_index(&self, v: u16) {
        self.bottom_index.set(v);
    }

    pub fn get_bottom_index(&self) -> u16 {
        self.bottom_index.get()
    }

    pub fn set_scroll_state_current(&self, v: i16) {
        self.scroll_state_current.set(v);
    }

    pub fn get_scroll_state_current(&self) -> i16 {
        self.scroll_state_current.get()
    }

    pub fn set_scroll_state_previous(&self, v: i16) {
        self.scroll_state_previous.set(v);
    }

    pub fn get_scroll_state_previous(&self) -> i16 {
        self.scroll_state_previous.get()
    }

    pub fn scrollGoingDown(&self) {
        self.set_scroll_state_current(self.get_scroll_state_current() + 1);
    }

    pub fn scrollGoingUp(&self) {
        self.set_scroll_state_current(self.get_scroll_state_current() - 1);
    }

    // To be called when a button is clicked on File Tab (left button of mouse)
    // offset_x , offset_y => Offset at which the button was clicked
    pub fn buttonClick(&mut self, offset_x: u16, offset_y: u16) {
        // X offset range, where when clicked we will assume its going for button click:w
        let clickableWidth = ((self.rect.width as f32 * 0.68) + 1f32) as u16..=((self.rect.width as f32 * 0.76) - 1f32) as u16;

        // Check if the clicked offset falls under the x offset of given range and the click has
        // happened within the files
        // + 3 => Skips the border of the top, one header row and blank row
        // - 2 => Skips the border and one spacing of the bottom
        let hasClickedOnDownload =
            clickableWidth.contains(&offset_x) && { offset_y >= self.rect.y + 3 } && { offset_y <= (self.rect.y + self.rect.height) - 2 };

        if hasClickedOnDownload {
            // Index of the clicked item
            // + 3 => Skips the border of the top, one header row and blank row
            let index = (self.get_top_index() + (offset_y - (self.rect.y + 3))) as usize;

            // Change the current should_download state of the File
            self.file.blocking_lock().inner_files.as_ref().unwrap()[index]
                .blocking_lock()
                .changeShouldDownload();
        }
    }
}

pub fn draw_files<B: Backend>(frame: &mut Frame<B>, size: Rect, scroll: &mut MutexGuard<FilesState>) {
    let download_yes = Cell::from("Yes").style(Style::default().bg(Color::Green).fg(Color::Black));
    let download_no = Cell::from("No").style(Style::default().bg(Color::Red).fg(Color::Black));

    let header_row = Row::new([Cell::from(NAME), Cell::from(TYPE), Cell::from(DOWNLOAD), Cell::from(PROGRESS)]);

    let blank_row = Row::new([""; 4]);

    // Run when it's the first draw of the files
    // TODO : Way to set the initial bottom index of the row(i.e how many rows to show) according
    // to the given size of the Files Tab
    // TODO : Way to re evaluate the bottom index when the screen resizes and size of the Files Tab
    // changes
    if scroll.get_top_index() == 0 && scroll.get_bottom_index() == 0 {
        let maxIndexOfRootFiles = scroll.file.blocking_lock().inner_files.as_ref().unwrap().len() as u16;
        let index = if maxIndexOfRootFiles < size.height - 4 {
            maxIndexOfRootFiles
        } else {
            size.height - 4
        };
        scroll.set_top_index(0);
        scroll.set_bottom_index(index);
        scroll.rect = size;
    }

    // Scroll UP
    if scroll.get_scroll_state_previous() > scroll.get_scroll_state_current() {
        // Scroll UP only when top index is greater than 0
        if scroll.get_top_index() > 0 {
            scroll.set_top_index(scroll.get_top_index() - 1);
            scroll.set_bottom_index(scroll.get_bottom_index() - 1);
        }

    // Scroll DOWN
    } else if scroll.get_scroll_state_previous() < scroll.get_scroll_state_current() {
        // Scroll UP only when bottom index is greater than total availaible rows
        let root_file = scroll.file.clone();
        if let Some(files) = &root_file.blocking_lock().inner_files {
            if scroll.get_bottom_index() < files.len() as u16 {
                scroll.set_top_index(scroll.get_top_index() + 1);
                scroll.set_bottom_index(scroll.get_bottom_index() + 1);
            }
        };
    }

    let createTableRow = |f: Arc<Mutex<File>>| -> Row {
        let name = { f.blocking_lock().name.clone() };
        let file_type = {
            match f.blocking_lock().file_type {
                FileType::REGULAR => String::from("File"),
                FileType::DIRECTORY => String::from("Folder"),
            }
        };

        let should_download = f.blocking_lock().should_download;

        Row::new(vec![
            Cell::from(name),
            Cell::from(file_type),
            if should_download { download_yes.clone() } else { download_no.clone() },
            Cell::from(format!("{} ", "NOTHING HERE")),
        ])
    };

    // Create the table rows to render
    let mut table_rows = vec![header_row.clone(), blank_row.clone()];
    for i in scroll.get_top_index()..scroll.get_bottom_index() {
        let file = scroll.file.blocking_lock().inner_files.as_ref().unwrap()[i as usize].clone();
        table_rows.push(createTableRow(file));
    }

    // Create the table
    let table = Table::new(table_rows)
        .widths(&[
            Constraint::Percentage(NAME_WIDTH_PERCENTAGE),
            Constraint::Percentage(TYPE_WIDTH_PERCENTAGE),
            Constraint::Percentage(DOWNLOAD_WIDTH_PERCENTAGE),
            Constraint::Percentage(PROGRESS_WIDTH_PERCENTAGE),
        ])
        .block(
            Block::default()
                .border_type(BorderType::Thick)
                .borders(Borders::ALL)
                .title(text::Span::styled(" File ", Style::default().fg(Color::Yellow)))
                .title_alignment(Alignment::Center),
        );

    // Render
    frame.render_widget(table, size);
}
