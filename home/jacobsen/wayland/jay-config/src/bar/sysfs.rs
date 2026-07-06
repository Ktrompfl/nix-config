/// Maps a 0..=100 percentage onto an index into an icon ladder of `len` icons.
pub fn icon_index(percent: u32, len: usize) -> usize {
    ((percent as usize) * (len - 1) / 100).min(len - 1)
}
