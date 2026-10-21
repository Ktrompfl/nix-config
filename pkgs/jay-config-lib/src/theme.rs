//! Read at runtime rather than compiled in, so that a theme change only needs
//! a config reload.

use std::{collections::HashMap, env, fs, path::PathBuf};

use jay_config::theme::{
    BarPosition, Color, colors, set_bar_font, set_bar_position, set_font, set_title_font, sized,
};

/// The flat `key = "value"` lines of `jay/theme.toml`.
fn load() -> HashMap<String, String> {
    let config_home = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")));
    let Some(text) =
        config_home.and_then(|dir| fs::read_to_string(dir.join("jay/theme.toml")).ok())
    else {
        log::warn!("could not read jay/theme.toml");
        return HashMap::new();
    };
    text.lines()
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| (key.trim().into(), value.trim().trim_matches('"').into()))
        .collect()
}

pub fn setup() {
    let theme = load();
    let color = |slot: &str| {
        let hex = theme.get(slot).map(String::as_str).unwrap_or_default();
        let component = |at: usize| {
            hex.get(at..at + 2)
                .and_then(|c| u8::from_str_radix(c, 16).ok())
                .unwrap_or(0)
        };
        Color::new(component(0), component(2), component(4))
    };

    let font = theme
        .get("monospace_font")
        .map(String::as_str)
        .unwrap_or("monospace");
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
