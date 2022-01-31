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

pub fn draw_files<B: Backend>(frame: &mut Frame<B>, size: Rect, scroll: &super::ui::Scroll) {
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
    let indexOffset;
    if scroll.getPrevious() > scroll.getCurrent() {
        indexOffset = scroll.getPrevious() - 1;
    } else {
        indexOffset = scroll.getCurrent();
    }
    let startIndex = 0 + indexOffset;
    let endIndex = 10 + indexOffset;
    let mut v = vec![header_row.clone(), header_row.clone()];
    for i in startIndex..endIndex {
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
