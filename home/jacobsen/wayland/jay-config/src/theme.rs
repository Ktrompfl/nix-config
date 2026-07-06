use jay_config::theme::{Color, colors, set_bar_font, set_font, set_title_font, sized};

use crate::generated::{BASE00, BASE01, BASE03, BASE05, BASE08, BASE09, BASE0A, BASE0B, BASE0D, MONOSPACE_FONT};

fn hex(s: &str) -> Color {
    let r = u8::from_str_radix(&s[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&s[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&s[4..6], 16).unwrap_or(0);
    Color::new(r, g, b)
}

pub fn setup() {
    set_font(MONOSPACE_FONT);
    set_title_font(MONOSPACE_FONT);
    set_bar_font(MONOSPACE_FONT);

    sized::BORDER_WIDTH.set(1);
    sized::TITLE_HEIGHT.set(16);
    sized::BAR_HEIGHT.set(16);
    sized::BAR_SEPARATOR_WIDTH.set(1);

    colors::BACKGROUND_COLOR.set_color(hex(BASE00));
    colors::BORDER_COLOR.set_color(hex(BASE03));

    colors::ATTENTION_REQUESTED_BACKGROUND_COLOR.set_color(hex(BASE09));
    colors::CAPTURED_FOCUSED_TITLE_BACKGROUND_COLOR.set_color(hex(BASE08));
    colors::CAPTURED_UNFOCUSED_TITLE_BACKGROUND_COLOR.set_color(hex(BASE0A));
    colors::FOCUSED_INACTIVE_TITLE_BACKGROUND_COLOR.set_color(hex(BASE03));
    colors::FOCUSED_INACTIVE_TITLE_TEXT_COLOR.set_color(hex(BASE05));
    colors::FOCUSED_TITLE_BACKGROUND_COLOR.set_color(hex(BASE0D));
    colors::FOCUSED_TITLE_TEXT_COLOR.set_color(hex(BASE00));
    colors::SEPARATOR_COLOR.set_color(hex(BASE03));
    colors::UNFOCUSED_TITLE_BACKGROUND_COLOR.set_color(hex(BASE03));
    colors::UNFOCUSED_TITLE_TEXT_COLOR.set_color(hex(BASE05));
    colors::HIGHLIGHT_COLOR.set_color(hex(BASE0B));

    colors::BAR_BACKGROUND_COLOR.set_color(hex(BASE01));
    colors::BAR_STATUS_TEXT_COLOR.set_color(hex(BASE05));
}
