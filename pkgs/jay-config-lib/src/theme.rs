//! Colors, fonts, and sizes, all of them following the active stylix scheme.
//!
//! They are read from `jay/theme.toml`, which home-manager writes (see
//! home/jacobsen/wayland/jay/default.nix), instead of being compiled into
//! config.so, so that switching stylix schemes only needs a config reload and
//! not a rebuild of this crate.

use std::{collections::HashMap, env, fs, path::PathBuf, sync::OnceLock};

use jay_config::theme::{
    BarPosition, Color, colors, set_bar_font, set_bar_position, set_font, set_title_font, sized,
};

const DEFAULTS: &[(&str, &str)] = &[
    ("base00", "181818"),
    ("base01", "282828"),
    ("base02", "383838"),
    ("base03", "585858"),
    ("base04", "b8b8b8"),
    ("base05", "d8d8d8"),
    ("base06", "e8e8e8"),
    ("base07", "f8f8f8"),
    ("base08", "ab4642"),
    ("base09", "dc9656"),
    ("base0a", "f7ca88"),
    ("base0b", "a1b56c"),
    ("base0c", "86c1b9"),
    ("base0d", "7cafc2"),
    ("base0e", "ba8baf"),
    ("base0f", "a16946"),
    ("monospace_font", "monospace"),
];

static THEME: OnceLock<HashMap<String, String>> = OnceLock::new();

fn config_path() -> Option<PathBuf> {
    let config_home = match env::var_os("XDG_CONFIG_HOME") {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(env::var_os("HOME")?).join(".config"),
    };
    Some(config_home.join("jay/theme.toml"))
}

/// Minimal parser for the flat `key = "value"` lines home-manager writes; no
/// nesting, arrays, or non-string values are needed here.
fn load() -> HashMap<String, String> {
    let Some(text) = config_path().and_then(|path| fs::read_to_string(path).ok()) else {
        return HashMap::new();
    };
    text.lines()
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| {
            (
                key.trim().to_string(),
                value.trim().trim_matches('"').to_string(),
            )
        })
        .collect()
}

/// The value of one of the keys default.nix writes, or its default.
pub fn string(key: &str) -> &'static str {
    THEME
        .get_or_init(load)
        .get(key)
        .map(String::as_str)
        .or_else(|| {
            DEFAULTS
                .iter()
                .find_map(|&(default, value)| (default == key).then_some(value))
        })
        .unwrap_or_default()
}

/// The color in one of the `RRGGBB` base16 slots.
fn color(slot: &str) -> Color {
    let hex = string(slot);
    let component = |at: usize| {
        hex.get(at..at + 2)
            .and_then(|c| u8::from_str_radix(c, 16).ok())
            .unwrap_or(0)
    };
    Color::new(component(0), component(2), component(4))
}

pub fn setup() {
    let font = string("monospace_font");
    set_font(font);
    set_title_font(font);
    set_bar_font(font);

    sized::BORDER_WIDTH.set(1);
    sized::TITLE_HEIGHT.set(16);
    sized::BAR_HEIGHT.set(16);
    sized::BAR_SEPARATOR_WIDTH.set(1);
    set_bar_position(BarPosition::Bottom);

    colors::BACKGROUND_COLOR.set_color(color("base00"));
    colors::BORDER_COLOR.set_color(color("base03"));

    colors::ATTENTION_REQUESTED_BACKGROUND_COLOR.set_color(color("base09"));
    colors::CAPTURED_FOCUSED_TITLE_BACKGROUND_COLOR.set_color(color("base08"));
    colors::CAPTURED_UNFOCUSED_TITLE_BACKGROUND_COLOR.set_color(color("base0a"));
    colors::FOCUSED_INACTIVE_TITLE_BACKGROUND_COLOR.set_color(color("base03"));
    colors::FOCUSED_INACTIVE_TITLE_TEXT_COLOR.set_color(color("base05"));
    colors::FOCUSED_TITLE_BACKGROUND_COLOR.set_color(color("base0d"));
    colors::FOCUSED_TITLE_TEXT_COLOR.set_color(color("base00"));
    colors::SEPARATOR_COLOR.set_color(color("base03"));
    colors::UNFOCUSED_TITLE_BACKGROUND_COLOR.set_color(color("base03"));
    colors::UNFOCUSED_TITLE_TEXT_COLOR.set_color(color("base05"));
    colors::HIGHLIGHT_COLOR.set_color(color("base0b"));

    colors::BAR_BACKGROUND_COLOR.set_color(color("base01"));
    colors::BAR_STATUS_TEXT_COLOR.set_color(color("base05"));
}
