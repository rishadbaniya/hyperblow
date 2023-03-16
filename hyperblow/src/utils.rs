// Some shared code that is usceful everywhere
// #![feature(async_closure)]
//
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
