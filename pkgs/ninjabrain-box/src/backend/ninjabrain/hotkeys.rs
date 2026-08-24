//! The keys the bot is driven by.
//!
//! The box binds these itself, in the preferences it writes, so they are both
//! what the box presses and what the bot listens for. Only the actions the box
//! actually drives are here; anything left out keeps the bot's default of -1,
//! which is no key at all.

/// One bound key: the preference it is stored under, the code the bot stores,
/// and the keysym the box presses.
pub struct Hotkey {
    pub name: &'static str,
    pub preference: &'static str,
    /// `location << 16 | virtual code`, which is how the bot stores a key on
    /// Linux. The virtual codes are JNativeHook's.
    pub code: i32,
    pub keysym: u32,
}

const STANDARD: i32 = 1 << 16;

/// F1 upwards.
///
/// These must stay within F1-F12. libuiohook turns an X keycode into a virtual
/// code through one of two tables, picked by whether the keymap's keycodes
/// component is named `evdev_*`; the tables agree below X keycode 97 and
/// diverge above it. F1-F12 are 67-76, 95 and 96, so they mean the same thing
/// whichever table is chosen. F13 upwards do not -- X keycode 119 is Delete
/// under one and F14 under the other.
pub const HOTKEYS: &[Hotkey] = &[
    Hotkey { name: "increment", preference: "hotkey_increment", code: STANDARD | 59, keysym: 0xffbe },
    Hotkey { name: "decrement", preference: "hotkey_decrement", code: STANDARD | 60, keysym: 0xffbf },
    Hotkey { name: "reset", preference: "hotkey_reset", code: STANDARD | 61, keysym: 0xffc0 },
    Hotkey { name: "undo", preference: "hotkey_undo", code: STANDARD | 62, keysym: 0xffc1 },
    Hotkey { name: "redo", preference: "hotkey_redo", code: STANDARD | 63, keysym: 0xffc2 },
];

/// Only ever called with names from the table above, so a miss is a bug.
pub fn get(name: &str) -> &'static Hotkey {
    HOTKEYS
        .iter()
        .find(|hotkey| hotkey.name == name)
        .expect("the hotkey exists")
}
