//TODO : Add a way to toggle the state of the Download

use tui::{
    backend::Backend,
    layout::{Constraint, Rect},
    style::Color,
    style::Style,
    widgets::{Cell, Row, Table},
    Frame,
};

use std::{cell, sync::MutexGuard};

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
    // Total size of the files table
    len: cell::Cell<u16>,

    // Scroll state
    // if current > previous then we can say user wanted to scroll down
    // if previous < current then we can say user wanted to scroll up
    // The difference is just 1 or -1
    scroll_state_current: std::cell::Cell<i16>,
    scroll_state_previous: std::cell::Cell<i16>,
    pub files: Vec<FileRow>,
    pub rect: Rect,
}

#[derive(Clone)]
pub struct FileRow {
    pub name: String,
    pub file_type: String,
    pub should_download: bool,
    pub total_size: String,
    pub total_downloaded: String,
}

impl FilesState {
    pub fn new() -> Self {
        FilesState {
            rect: Rect::new(0, 0, 0, 0),
            top_index: cell::Cell::new(0),
            bottom_index: cell::Cell::new(0),
            len: cell::Cell::new(0),
            scroll_state_current: cell::Cell::new(0),
            scroll_state_previous: cell::Cell::new(0),
            files: Vec::new(),
        }
    }

    pub fn add_file(&mut self, v: FileRow) {
        self.files.push(v);
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
        let clickable_width =
            (self.rect.width as f32 * 0.68) as u16..=(self.rect.width as f32 * 0.76) as u16;
        let has_clicked_on_download =
            clickable_width.contains(&offset_x) && { offset_y >= self.rect.y + 2 };

        if has_clicked_on_download {
            let indexOffset = (self.get_top_index() + (offset_y - (self.rect.y + 2))) as usize;
            self.files[indexOffset].should_download = !self.files[indexOffset].should_download;
        }
    }
}

pub fn draw_files<B: Backend>(
    frame: &mut Frame<B>,
    size: Rect,
    scroll: &mut MutexGuard<FilesState>,
) {
    let download_yes = Cell::from("Yes").style(Style::default().bg(Color::Green).fg(Color::Black));
    let download_no = Cell::from("No").style(Style::default().bg(Color::Red).fg(Color::Black));

    let header_row = Row::new([
        Cell::from(NAME),
        Cell::from(TYPE),
        Cell::from(DOWNLOAD),
        Cell::from(PROGRESS),
    ]);

    let blank_row = Row::new([""; 4]);

    let file = FileRow {
        name: String::from("0.0 - Introduction to some of the files here"),
        file_type: String::from("Folder"),
        should_download: false,
        total_size: String::from("0.1Mb"),
        total_downloaded: String::from("0.1Mb"),
    };
    let file1 = FileRow {
        name: String::from("0.1 - Kernel data structure formats"),
        file_type: String::from("Folder"),
        should_download: true,
        total_size: String::from("10Mb"),
        total_downloaded: String::from("20Mb"),
    };

    // Run when it's the first draw of the files
    // TODO : Way to set the initial bottom index of the row(i.e how many rows to show) according
    // to the given size of the Files Tab
    // TODO : Way to re evaluate the bottom index when the screen resizes and size of the Files Tab
    // changes
    if scroll.get_top_index() == 0 && scroll.get_bottom_index() == 0 {
        scroll.set_top_index(0);
        scroll.set_bottom_index(10);
        for i in 1..10 {
            scroll.add_file(file.clone());
        }
        for i in 1..10 {
            scroll.add_file(file1.clone());
        }
        for i in 1..10 {
            scroll.add_file(file.clone());
        }
        for i in 1..10 {
            scroll.add_file(file1.clone());
        }
        for i in 1..10 {
            scroll.add_file(file.clone());
        }
        for i in 1..10 {
            scroll.add_file(file1.clone());
        }
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
        if scroll.get_bottom_index() < scroll.files.len() as u16 {
            scroll.set_top_index(scroll.get_top_index() + 1);
            scroll.set_bottom_index(scroll.get_bottom_index() + 1);
        }
    }

    let createTableRow = |f: FileRow| -> Row {
        let p: bool = ((scroll.rect.width as f32 * 0.68) as u16
            ..=(scroll.rect.width as f32 * 0.76) as u16)
            .contains(&130);
        Row::new(vec![
            Cell::from(f.name),
            Cell::from(f.file_type),
            if f.should_download {
                download_yes.clone()
            } else {
                download_no.clone()
            },
            Cell::from(format!("{:?} ", p)),
        ])
    };

    let mut v = vec![header_row.clone(), blank_row.clone()];
    for i in scroll.get_top_index()..scroll.get_bottom_index() {
        v.push(createTableRow(scroll.files[i as usize].clone()));
    }

    // Create the table
    let table = Table::new(v).widths(&[
        Constraint::Percentage(NAME_WIDTH_PERCENTAGE),
        Constraint::Percentage(TYPE_WIDTH_PERCENTAGE),
        Constraint::Percentage(DOWNLOAD_WIDTH_PERCENTAGE),
        Constraint::Percentage(PROGRESS_WIDTH_PERCENTAGE),
    ]);

    // Render
    frame.render_widget(table, size);
}
