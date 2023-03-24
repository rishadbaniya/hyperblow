// Some shared code that is useful everywhere
//
// As of right now, i don't know the proper structure on where the common code should be placed, so
// i'm placing it right here
//
pub fn bytes_to_human_readable(size: usize) -> String {
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

#[macro_export]
macro_rules! ACell {
    ($e : expr) => {
        AtomicCell::new($e)
    };
}
