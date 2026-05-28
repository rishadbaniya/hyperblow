pub struct ByteSizeFormatter;

impl ByteSizeFormatter {
    pub fn human_readable(size: usize) -> String {
        let kibibytes = size as f32 / 1024_f32;
        if kibibytes < 1024_f32 {
            format!("{:.2} KiB", kibibytes)
        } else {
            let mibibytes = kibibytes / 1024_f32;
            if mibibytes < 1024_f32 {
                format!("{:.2} MiB", mibibytes)
            } else {
                let gibibytes = mibibytes / 1024_f32;
                format!("{:.2} GiB", gibibytes)
            }
        }
    }
}

#[macro_export]
macro_rules! ACell {
    ($e : expr) => {
        AtomicCell::new($e)
    };
}
