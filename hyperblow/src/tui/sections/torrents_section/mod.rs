// There might be some debate on using Table, rather than customly creating each column with a
// certain size. One point to be noted is that we can't create wigets like Gauge inside of a Table
// widget, only text data can be rendered inside of Table widget

//use super::{mouse::MouseEv, tabs::bandwidth_tab::TabSectionBandwidth};
use crate::tui::tui_state::TUIState;
use crate::utils;
use std::{fmt::format, io::stdout, ops::Range, rc::Rc, sync::Arc, time::Duration};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::Alignment,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    style::{Color, Modifier},
    terminal::Frame,
    terminal::Terminal,
    text::Span,
    text::Spans,
    widgets::{Block, BorderType, Borders, Cell, Gauge, List, ListItem, Row, Table},
};

// Constants that define, the division percentage of the column in
// Torrents Section of TUI
// Eg.
//
//     NAME = "Name" -- It's the name of the header of the column
//     NAME_PERC = 30 -- It's the percentage width of the entire provided
//                       area that column "Name" is gonna occupy
//
const NAME: &str = "Name";
const NAME_PERC: u16 = 40;

const PROGRESS: &str = "Progress";
const PROGRESS_PERC: u16 = 10;

const STATUS: &str = "Status";
const STATUS_PERC: u16 = 10;

const BYTES: &str = "Status";
const BYTES_PERC: u16 = 10;

const IN: &str = "In";
const IN_PERC: u16 = 10;

const OUT: &str = "Out";
const OUT_PERC: u16 = 10;

const TIME_LEFT: &str = "Time Left";
const TIME_LEFT_PERC: u16 = 10;

pub struct TorrentsSection;

impl TorrentsSection {
    pub(crate) fn draw<B: Backend>(frame: &mut Frame<B>, area: Rect, state: Rc<TUIState>) {
        //let download_yes = Cell::from("Yes").style(Style::default().bg(Color::Green).fg(Color::Black));
        //let download_no = Cell::from("No").style(Style::default().bg(Color::Red).fg(Color::Black));

        // Create and render the border
        let border = Block::default()
            .border_type(BorderType::Thick)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(Span::styled(" Hyperblow ", Style::default().fg(Color::Yellow)))
            .title_alignment(Alignment::Center);

        frame.render_widget(border, area.clone());
        let area = Layout::default().constraints([Constraint::Min(0)]).margin(2).split(area)[0];
        let area = Layout::default().constraints([Constraint::Length(2), Constraint::Min(0)]).split(area);

        Self::draw_header_column(frame, area[0]);
        Self::draw_torrents_columns(frame, area[1], state.clone());
    }

    // Displays only the header column
    fn draw_header_column<B: Backend>(frame: &mut Frame<B>, area: Rect) {
        let table = Table::new(
            [Row::new(vec![NAME, PROGRESS, STATUS, BYTES, IN, OUT, TIME_LEFT])], //Row::new([Cell::from("ABC")),
        )
        .widths(&[
            Constraint::Percentage(NAME_PERC),
            Constraint::Percentage(PROGRESS_PERC),
            Constraint::Percentage(STATUS_PERC),
            Constraint::Percentage(BYTES_PERC),
            Constraint::Percentage(IN_PERC),
            Constraint::Percentage(OUT_PERC),
            Constraint::Percentage(TIME_LEFT_PERC),
        ]);
        frame.render_widget(table, area);
    }

    // Displays the contents relatable to header column
    fn draw_torrents_columns<B: Backend>(frame: &mut Frame<B>, area: Rect, state: Rc<TUIState>) {
        // Height of the column of each item in torrens section
        const ROW_HEIGHT: u16 = 1;

        // Divides the given "area" into n possible rows of COLUMN_HEIGHT that have 7 columns
        let column_areas: Vec<Vec<Rect>> = {
            let mut columns_area: Vec<Vec<Rect>> = Vec::default();
            let (x, mut y) = (area.x, area.y);
            let (width, height) = (area.width, area.height);
            let initial_y = y;

            while y < initial_y + height {
                let row_area = Rect {
                    x,
                    y,
                    height: ROW_HEIGHT,
                    width,
                };
                let column_area = Layout::default()
                    .direction(Direction::Horizontal)
                    .horizontal_margin(1)
                    .constraints([
                        Constraint::Percentage(NAME_PERC),
                        Constraint::Percentage(PROGRESS_PERC),
                        Constraint::Percentage(STATUS_PERC),
                        Constraint::Percentage(BYTES_PERC),
                        Constraint::Percentage(IN_PERC),
                        Constraint::Percentage(OUT_PERC),
                        Constraint::Percentage(TIME_LEFT_PERC),
                    ])
                    .split(row_area);
                y = y + ROW_HEIGHT;
                columns_area.push(column_area);
            }
            columns_area
        };

        //println!("{}", column_areas.len());
        let ref torrent_handles = *state.engine.torrents.blocking_lock();

        for (index, handle) in torrent_handles.iter().enumerate() {
            let name = Block::default().title(handle.name());
            //println!("{:?}", column_areas[0][0]);
            //
            let progress = Gauge::default()
                .percent(1)
                .gauge_style(Style::default().fg(Color::White).bg(Color::Black).add_modifier(Modifier::BOLD))
                .percent(20);
            frame.render_widget(name, column_areas[index][0]);
            frame.render_widget(progress.clone(), column_areas[index][1]);
            frame.render_widget(progress, column_areas[index][2]);
        }

        //let progress = Gauge::default().percent(percent);
        //frame.render_widget(progress, columns[6]);
    }
}
