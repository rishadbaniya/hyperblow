use std::sync::Arc;

use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Axis, Block, Chart, Dataset, GraphType},
    Frame,
};

use crate::core::File;
use tokio::sync::Mutex;

use std::cell::RefCell;
use std::rc::Rc;

/// Data for the Bandwidth Tab Section of TUI
pub struct TabSectionFiles {
    pub file_tree: Arc<Mutex<File>>,

    pub widgets: Rc<RefCell<Vec<Block<'static>>>>,
}

impl TabSectionFiles {
    pub fn renderWidget<B: Backend>(&self, frame: &mut Frame<B>, area: Rect) {
        let chunks = Layout::default()
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .direction(Direction::Vertical)
            .split(area);

        for (ind, b) in self.widgets.borrow().iter().enumerate() {
            if ind < 30 {
                frame.render_widget(b.clone(), chunks[ind]);
            } else {
                break;
            }
        }
    }
    pub async fn loadWidgets(&self) {
        let x = self.file_tree.lock().await.tabs_traverse_names(0).await;
        for i in x {
            self.widgets.borrow_mut().push(Block::default().title(i));
        }
    }
}
