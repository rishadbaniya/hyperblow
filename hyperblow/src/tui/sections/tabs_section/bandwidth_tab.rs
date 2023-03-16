use tui::{
    backend::Backend,
    layout::Rect,
    style::{Color, Style},
    symbols::Marker,
    text::Span,
    widgets::{Axis, Block, Chart, Dataset, GraphType},
    Frame,
};

/// Data for the Bandwidth Tab Section of TUI
pub struct TabSectionBandwidth {
    // Download speed in bytes/s
    pub download_speed: usize,

    // Upload speed in bytes/s
    pub upload_speed: usize,
}

impl TabSectionBandwidth {
    pub fn renderWidget<B: Backend>(&self, frame: &mut Frame<B>, area: Rect) {
        let datasets = vec![
            Dataset::default()
                .name("Upload Speed : KiB/s")
                .marker(Marker::Dot)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(Color::Cyan))
                .data(&[(0.0, 5.0), (1.0, 6.0), (1.5, 6.434)]),
            Dataset::default()
                .name("Download Speed : MiB/s")
                .marker(Marker::Dot)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(Color::Magenta))
                .data(&[(4.0, 5.0), (5.0, 8.0), (7.66, 13.5)]),
        ];

        let widget_download_bandwidth_chart = Chart::new(datasets)
            .x_axis(
                Axis::default()
                    .title(Span::styled("Time", Style::default().fg(Color::Red)))
                    .style(Style::default().fg(Color::White))
                    .bounds([0.0, 10.0])
                    .labels(["0.0", "5.0", "10.0"].iter().cloned().map(Span::from).collect()),
            )
            .y_axis(
                Axis::default()
                    .title(Span::styled("Bandwidth", Style::default().fg(Color::Red)))
                    .style(Style::default().fg(Color::White))
                    .bounds([0.0, 10.0])
                    .labels(["0.0", "5.0", "10.0"].iter().cloned().map(Span::from).collect()),
            );

        frame.render_widget(widget_download_bandwidth_chart, area);
    }
}
