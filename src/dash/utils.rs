// BUG: https://github.com/emarsden/dash-mpd-rs/blob/main/src/lib.rs#L190-L231
pub(super) fn iso8601_duration_to_seconds(duration: &str) -> Result<f32, String> {
    match iso8601::duration(duration)? {
        iso8601::Duration::YMDHMS {
            year,
            month,
            day,
            hour,
            minute,
            second,
            millisecond,
        } => Ok((year as f32 * 365.0 * 31.0 * 24.0 * 60.0 * 60.0)
            + (month as f32 * 31.0 * 24.0 * 60.0 * 60.0)
            + (day as f32 * 24.0 * 60.0 * 60.0)
            + (hour as f32 * 60.0 * 60.0)
            + (minute as f32 * 60.0)
            + second as f32
            + (millisecond as f32 * 0.001)),
        iso8601::Duration::Weeks(w) => Ok(w as f32 * 60.0 * 60.0 * 24.0 * 7.0),
    }
}
