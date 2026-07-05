use std::{fs, path::Path};

/// Shared helpers for the `/proc` and `/sys` based bar modules (battery,
/// backlight, network).
pub fn read_trim(path: impl AsRef<Path>) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

/// Maps a 0..=100 percentage onto an index into an icon ladder of `len` icons.
pub fn icon_index(percent: u32, len: usize) -> usize {
    ((percent as usize) * (len - 1) / 100).min(len - 1)
}
