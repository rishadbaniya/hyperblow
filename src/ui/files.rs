use tui::{
    backend::Backend,
    layout::{Constraint, Rect},
    style::Color,
    style::Style,
    widgets::{Cell, Row, Table},
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

pub fn draw_files<B: Backend>(frame: &mut Frame<B>, size: Rect, scroll: &super::ui::FilesState) {
    let download_yes = Cell::from("Yes").style(Style::default().bg(Color::Green).fg(Color::Black));
    let download_no = Cell::from("Yes").style(Style::default().bg(Color::Red).fg(Color::Black));

    let header_row = Row::new([
        Cell::from(NAME),
        Cell::from(TYPE),
        Cell::from(DOWNLOAD),
        Cell::from(PROGRESS),
    ]);

    let blank_row = Row::new([""; 4]);

    let row01 = Row::new(vec![
        Cell::from("00 - Introduction to some of the files here"),
        Cell::from("Folder"),
        download_yes.clone(),
        Cell::from("10.2 MB/ 200MB | 29.1%  "),
    ]);
    let row1 = Row::new(vec![
        Cell::from("01 - WHat is going on here"),
        Cell::from("Folder"),
        download_no.clone(),
        Cell::from("10.2 MB/ 200MB | 20.1%  "),
    ]);

    let allRows = vec![
        row01.clone(),
        row01.clone(),
        row01.clone(),
        row01.clone(),
        row01.clone(),
        row01.clone(),
        row1.clone(),
        row1.clone(),
        row1.clone(),
        row1.clone(),
        row1.clone(),
        row1.clone(),
        row01.clone(),
        row01.clone(),
        row01.clone(),
        row01.clone(),
        row01.clone(),
        row01.clone(),
        row1.clone(),
        row1.clone(),
        row1.clone(),
        row1.clone(),
        row1.clone(),
        row1.clone(),
        row01.clone(),
        row01.clone(),
        row01.clone(),
        row01.clone(),
        row01.clone(),
        row01.clone(),
        row1.clone(),
        row1.clone(),
        row1.clone(),
        row1.clone(),
        row1.clone(),
        row1.clone(),
    ];

    // Run when it's the first draw of the files
    if scroll.get_top_index() == 0 && scroll.get_bottom_index() == 0 {
        scroll.set_top_index(0);
        scroll.set_bottom_index(10);
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
        // Scroll UP only when top index is greater than total availaible rows
        if scroll.get_bottom_index() < allRows.len() as u16 {
            scroll.set_top_index(scroll.get_top_index() + 1);
            scroll.set_bottom_index(scroll.get_bottom_index() + 1);
        }
    }

    let mut v = vec![header_row.clone(), blank_row.clone()];
    for i in scroll.get_top_index()..scroll.get_bottom_index() {
        v.push(allRows[i as usize].clone());
    }

    let table = Table::new(v).widths(&[
        Constraint::Percentage(NAME_WIDTH_PERCENTAGE),
        Constraint::Percentage(TYPE_WIDTH_PERCENTAGE),
        Constraint::Percentage(DOWNLOAD_WIDTH_PERCENTAGE),
        Constraint::Percentage(PROGRESS_WIDTH_PERCENTAGE),
    ]);
    frame.render_widget(table, size);
}
