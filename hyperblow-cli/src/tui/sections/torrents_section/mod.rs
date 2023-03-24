// There might be some debate on using Table, rather than customly creating each column with a
// certain size. One point to be noted is that we can't create wigets like Gauge inside of a Table
// widget, only text data can be rendered inside of Table widget

//use super::{mouse::MouseEv, tabs::bandwidth_tab::TabSectionBandwidth};
use crate::{core::tracker::Tracker, tui::tui_state::TUIState, utils};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    terminal::Frame,
    text::Span,
    widgets::{Block, BorderType, Borders, Cell, Gauge, List, ListItem, Row, Table},
};
use std::{
    fmt::{format, Display},
    rc::Rc,
};

/// Constants that define, the division percentage of the column in
/// Torrents Section of TUI
/// Eg.
///
///     NAME = "Name" -- It's the name of the header of the column
///     NAME_PERC = 30 -- It's the percentage width of the entire provided
///                       area that column "Name" is gonna occupy
///
const NAME: &str = "Name";
const NAME_PERC: u16 = 35;

const PROGRESS: &str = "Progress";
const PROGRESS_PERC: u16 = 10;

const STATUS: &str = "Status";
const STATUS_PERC: u16 = 14;

const BYTES: &str = "Bytes";
const BYTES_PERC: u16 = 10;

const IN: &str = "In";
const IN_PERC: u16 = 10;

const OUT: &str = "Out";
const OUT_PERC: u16 = 10;

const TIME_LEFT: &str = "Time Left";
const TIME_LEFT_PERC: u16 = 10;

pub struct TorrentsSection;

impl TorrentsSection {
    /// Draws the Torrents Section columns
    ///
    /// It includes info such as
    /// - Name of the torrent
    /// - Progress of the torrent file in percentage,
    /// - Status : Downloading, Seeding, Paused
    /// - Bytes : "12 GiB / 20 GiB" Total Bytes Downloaded
    /// - In - "12 GiB/s" Total Download Speed
    /// - Out - "1 GiB/s" Total Upload Speed
    /// - Time Left - "00:02:12" Time Left in HH:MM:SS
    pub(crate) fn draw<B: Backend>(frame: &mut Frame<B>, area: Rect, state: Rc<TUIState>) {
        // Create and render the border first
        let border = Block::default()
            .border_type(BorderType::Thick)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(Span::styled(" Hyperblow ", Style::default().fg(Color::Yellow)))
            .title_alignment(Alignment::Center);
        frame.render_widget(border, area.clone());

        // Recalculate the area after border is built
        let area = Layout::default()
            .constraints([Constraint::Min(0)])
            .margin(2)
            .split(area)[0];

        // Split the area for header row and torrents row
        let area = Layout::default()
            .constraints([Constraint::Length(2), Constraint::Min(0)])
            .split(area);

        // Draw the header and torrents column
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
                    .split(row_area)
                    .iter()
                    .cloned()
                    .collect();
                y = y + ROW_HEIGHT;
                columns_area.push(column_area);
            }
            columns_area
        };

        //println!("{}", column_areas.len());
        let ref torrent_handles = *state.engine.torrents.blocking_lock();

        for (index, handle) in torrent_handles.iter().enumerate() {
            // Widget to dispaly the full name of the torrent downloaded
            let widget_name = {
                let name = handle.name();
                Block::default().title(name).title_alignment(Alignment::Left)
            };

            // Widget to display progress in percentage
            let widget_progress = {
                let progress_perc = 10;
                Gauge::default()
                    .percent(1)
                    .gauge_style(
                        Style::default()
                            .fg(Color::White)
                            .bg(Color::Black)
                            .add_modifier(Modifier::BOLD),
                    )
                    .percent(progress_perc)
            };

            // Widget to display status to show either the torrent session is Paused, Downloading
            // or Seeding
            let widget_status = {
                let status = TorrentStatus::Downloading;
                let (title, fg_color) = match status {
                    TorrentStatus::Downloading => (status.to_string(), Color::Green),
                    TorrentStatus::Seeding => (status.to_string(), Color::Red),
                    TorrentStatus::Paused => (status.to_string(), Color::Blue),
                };
                Block::default()
                    .title(title)
                    .title_alignment(Alignment::Center)
                    .style(Style::default().bg(fg_color).fg(Color::Black))
            };

            // Widget to display the Amount of data downloaded Of Total data
            let widget_bytes = {
                let bytes_complete = utils::bytes_to_human_readable(handle.bytes_complete());
                let bytes_total = utils::bytes_to_human_readable(handle.bytes_total());

                Block::default()
                    .title(format!("{bytes_complete}/{bytes_total}"))
                    .title_alignment(Alignment::Left)
            };

            // Widget to display the Download speed
            let widget_in = {
                let download_speed = utils::bytes_to_human_readable(handle.download_speed());

                Block::default().title(format!("{download_speed}/s"))
            };

            // Widget to display the Upload speed
            let widget_out = {
                let upload_speed = utils::bytes_to_human_readable(handle.upload_speed());
                Block::default()
                    .title(format!("{upload_speed}/s"))
                    .title_alignment(Alignment::Left)
            };

            let widget_time_left = {
                Block::default()
                    .title(format!("00:20:21"))
                    .title_alignment(Alignment::Left)
            };

            // Render all widgets
            frame.render_widget(widget_name, column_areas[index][0]);
            frame.render_widget(widget_progress, column_areas[index][1]);
            frame.render_widget(widget_status, column_areas[index][2]);
            frame.render_widget(widget_bytes, column_areas[index][3]);
            frame.render_widget(widget_in, column_areas[index][4]);
            frame.render_widget(widget_out, column_areas[index][5]);
            frame.render_widget(widget_time_left, column_areas[index][6]);
        }
    }
}

enum TorrentStatus {
    Downloading,
    Seeding,
    Paused,
}

impl Display for TorrentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            TorrentStatus::Downloading => write!(f, "Downloading"),
            TorrentStatus::Seeding => write!(f, "Seeding"),
            TorrentStatus::Paused => write!(f, "Paused"),
        }?;
        Ok(())
    }
}
