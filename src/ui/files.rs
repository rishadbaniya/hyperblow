//TODO : Add a way to toggle the state of the Download

use tui::{
    backend::Backend,
    layout::{Constraint, Rect},
    style::Color,
    style::Style,
    widgets::{Cell, Row, Table},
    Frame,
};

use crate::ui::ui::FileRow;

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

pub fn draw_files<B: Backend>(
    frame: &mut Frame<B>,
    size: Rect,
    scroll: &mut super::ui::FilesState,
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

    let file = super::ui::FileRow {
        name: String::from("0.0 - Introduction to some of the files here"),
        file_type: String::from("Folder"),
        should_download: false,
        total_size: String::from("0.1Mb"),
        total_downloaded: String::from("0.1Mb"),
    };
    let file1 = super::ui::FileRow {
        name: String::from("0.0 - Introduction to some of the files here"),
        file_type: String::from("Folder"),
        should_download: true,
        total_size: String::from("0.1Mb"),
        total_downloaded: String::from("0.1Mb"),
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
