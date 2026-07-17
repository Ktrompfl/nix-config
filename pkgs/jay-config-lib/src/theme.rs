// Not every base16 slot has a consumer yet, but all 16 are kept available
// here so bar/window modules can reach for any of them without adding a new
// field.
#![allow(dead_code)]

use std::{env, fs, path::PathBuf, sync::OnceLock};

use jay_config::theme::{Color, colors, set_bar_font, set_font, set_title_font, sized};

/// Colors and fonts here mirror the active stylix scheme, but are read from
/// `jay/theme.toml` (written by home-manager, see `home/jacobsen/wayland/jay.nix`)
/// rather than compiled in, so switching schemes only needs a config reload
/// (`reload()`, bound in shortcuts.rs), not a rebuild of this crate.
struct Theme {
    base00: String,
    base01: String,
    base02: String,
    base03: String,
    base04: String,
    base05: String,
    base06: String,
    base07: String,
    base08: String,
    base09: String,
    base0a: String,
    base0b: String,
    base0c: String,
    base0d: String,
    base0e: String,
    base0f: String,
    monospace_font: String,
    cursor_theme: String,
    cursor_size: String,
    gtk_theme: String,
    qt_platform_theme: String,
    qt_style: String,
}

impl Theme {
    /// base16 default-dark (Chris Kempson), used whenever `theme.toml` or a
    /// key in it is missing, e.g. before home-manager has ever written it.
    fn defaults() -> Self {
        Self {
            base00: "181818".to_string(),
            base01: "282828".to_string(),
            base02: "383838".to_string(),
            base03: "585858".to_string(),
            base04: "b8b8b8".to_string(),
            base05: "d8d8d8".to_string(),
            base06: "e8e8e8".to_string(),
            base07: "f8f8f8".to_string(),
            base08: "ab4642".to_string(),
            base09: "dc9656".to_string(),
            base0a: "f7ca88".to_string(),
            base0b: "a1b56c".to_string(),
            base0c: "86c1b9".to_string(),
            base0d: "7cafc2".to_string(),
            base0e: "ba8baf".to_string(),
            base0f: "a16946".to_string(),
            monospace_font: "monospace".to_string(),
            cursor_theme: "Adwaita".to_string(),
            cursor_size: "24".to_string(),
            gtk_theme: "Adwaita".to_string(),
            qt_platform_theme: "gtk3".to_string(),
            qt_style: "Adwaita".to_string(),
        }
    }

    fn set(&mut self, key: &str, value: String) {
        let field = match key {
            "base00" => &mut self.base00,
            "base01" => &mut self.base01,
            "base02" => &mut self.base02,
            "base03" => &mut self.base03,
            "base04" => &mut self.base04,
            "base05" => &mut self.base05,
            "base06" => &mut self.base06,
            "base07" => &mut self.base07,
            "base08" => &mut self.base08,
            "base09" => &mut self.base09,
            "base0a" => &mut self.base0a,
            "base0b" => &mut self.base0b,
            "base0c" => &mut self.base0c,
            "base0d" => &mut self.base0d,
            "base0e" => &mut self.base0e,
            "base0f" => &mut self.base0f,
            "monospace_font" => &mut self.monospace_font,
            "cursor_theme" => &mut self.cursor_theme,
            "cursor_size" => &mut self.cursor_size,
            "gtk_theme" => &mut self.gtk_theme,
            "qt_platform_theme" => &mut self.qt_platform_theme,
            "qt_style" => &mut self.qt_style,
            _ => return,
        };
        *field = value;
    }

    fn load() -> Self {
        let mut theme = Self::defaults();
        if let Some(text) = config_path().and_then(|path| fs::read_to_string(path).ok()) {
            for (key, value) in parse(&text) {
                theme.set(key, value);
            }
        }
        theme
    }
}

fn config_path() -> Option<PathBuf> {
    if let Some(dir) = env::var_os("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(dir).join("jay/theme.toml"));
    }
    let home = env::var_os("HOME")?;
    Some(PathBuf::from(home).join(".config/jay/theme.toml"))
}

/// Minimal parser for the flat `key = "value"` lines home-manager writes;
/// no nesting, arrays, or non-string values are needed here.
fn parse(text: &str) -> impl Iterator<Item = (&str, String)> {
    text.lines().filter_map(|line| {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            return None;
        }
        let (key, value) = line.split_once('=')?;
        Some((key.trim(), value.trim().trim_matches('"').to_string()))
    })
}

static THEME: OnceLock<Theme> = OnceLock::new();

fn get() -> &'static Theme {
    THEME.get_or_init(Theme::load)
}

fn hex(s: &str) -> Color {
    let r = u8::from_str_radix(&s[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&s[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&s[4..6], 16).unwrap_or(0);
    Color::new(r, g, b)
}

pub fn setup() {
    let theme = get();

    set_font(&theme.monospace_font);
    set_title_font(&theme.monospace_font);
    set_bar_font(&theme.monospace_font);

    sized::BORDER_WIDTH.set(1);
    sized::TITLE_HEIGHT.set(16);
    sized::BAR_HEIGHT.set(16);
    sized::BAR_SEPARATOR_WIDTH.set(1);

    colors::BACKGROUND_COLOR.set_color(hex(&theme.base00));
    colors::BORDER_COLOR.set_color(hex(&theme.base03));

    colors::ATTENTION_REQUESTED_BACKGROUND_COLOR.set_color(hex(&theme.base09));
    colors::CAPTURED_FOCUSED_TITLE_BACKGROUND_COLOR.set_color(hex(&theme.base08));
    colors::CAPTURED_UNFOCUSED_TITLE_BACKGROUND_COLOR.set_color(hex(&theme.base0a));
    colors::FOCUSED_INACTIVE_TITLE_BACKGROUND_COLOR.set_color(hex(&theme.base03));
    colors::FOCUSED_INACTIVE_TITLE_TEXT_COLOR.set_color(hex(&theme.base05));
    colors::FOCUSED_TITLE_BACKGROUND_COLOR.set_color(hex(&theme.base0d));
    colors::FOCUSED_TITLE_TEXT_COLOR.set_color(hex(&theme.base00));
    colors::SEPARATOR_COLOR.set_color(hex(&theme.base03));
    colors::UNFOCUSED_TITLE_BACKGROUND_COLOR.set_color(hex(&theme.base03));
    colors::UNFOCUSED_TITLE_TEXT_COLOR.set_color(hex(&theme.base05));
    colors::HIGHLIGHT_COLOR.set_color(hex(&theme.base0b));

    colors::BAR_BACKGROUND_COLOR.set_color(hex(&theme.base01));
    colors::BAR_STATUS_TEXT_COLOR.set_color(hex(&theme.base05));
}

pub fn base00() -> &'static str {
    &get().base00
}

pub fn base01() -> &'static str {
    &get().base01
}

pub fn base02() -> &'static str {
    &get().base02
}

pub fn base03() -> &'static str {
    &get().base03
}

pub fn base04() -> &'static str {
    &get().base04
}

pub fn base05() -> &'static str {
    &get().base05
}

pub fn base06() -> &'static str {
    &get().base06
}

pub fn base07() -> &'static str {
    &get().base07
}

pub fn base08() -> &'static str {
    &get().base08
}

pub fn base09() -> &'static str {
    &get().base09
}

pub fn base0a() -> &'static str {
    &get().base0a
}

pub fn base0b() -> &'static str {
    &get().base0b
}

pub fn base0c() -> &'static str {
    &get().base0c
}

pub fn base0d() -> &'static str {
    &get().base0d
}

pub fn base0e() -> &'static str {
    &get().base0e
}

pub fn base0f() -> &'static str {
    &get().base0f
}

pub fn cursor_theme() -> &'static str {
    &get().cursor_theme
}

pub fn cursor_size() -> &'static str {
    &get().cursor_size
}

pub fn gtk_theme() -> &'static str {
    &get().gtk_theme
}

pub fn qt_platform_theme() -> &'static str {
    &get().qt_platform_theme
}

pub fn qt_style() -> &'static str {
    &get().qt_style
}
