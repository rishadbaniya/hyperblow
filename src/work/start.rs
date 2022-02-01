use crate::ui::files::FilesState;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

// Starting Point for the working thread
pub fn start(fileState: Arc<Mutex<FilesState>>) {
    thread::sleep(Duration::from_millis(200));
    fileState.lock().unwrap().files[0].should_download = true;
}
