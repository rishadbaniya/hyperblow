use std::cell::Cell;

// Struct that stores the offset of mouse everytime we move the cursor
// Note : Used to store the mouse offset as a global state
pub struct MouseOffset {
    // Offset in (x, y) format
    pub offset: (Cell<u16>, Cell<u16>),
}

impl MouseOffset {
    // Used to create MouseOffset instance initially
    pub fn default() -> Self {
        Self {
            offset: (Cell::new(0), Cell::new(0)),
        }
    }

    pub fn get_x(&self) -> u16 {
        self.offset.0.get()
    }

    pub fn get_y(&self) -> u16 {
        self.offset.1.get()
    }

    pub fn set_x(&self, x: u16) {
        self.offset.0.set(x);
    }

    pub fn set_y(&self, y: u16) {
        self.offset.1.set(y);
    }
}
