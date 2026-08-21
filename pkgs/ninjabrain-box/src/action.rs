//! Everything the overlay can be told to do, from the command line or from a
//! compositor binding.

/// One of the bot's own actions.
#[derive(Debug, PartialEq, Eq)]
pub struct BotAction {
    /// What it is called on the command line.
    pub name: &'static str,
    /// The preference its hotkey is bound under in the file written for the
    /// bot, which is what makes the key below mean this action.
    pub preference: &'static str,
    /// The key it is bound to, as the bot stores one on Linux:
    /// `location << 16 | evdev code`, which for these is exactly what
    /// JNativeHook reports for the function keys, F1 upwards.
    pub code: i32,
    /// The same key as a keysym, which is what the box looks up to find the
    /// keycode to replay. Nothing but the box can reach that display, so the
    /// function keys are free whatever the game does with them.
    pub keysym: u32,
}

const STANDARD: i32 = 1 << 16;

pub const BOT_ACTIONS: &[BotAction] = &[
    BotAction { name: "increment", preference: "hotkey_increment", code: STANDARD | 59, keysym: 0xffbe },
    BotAction { name: "decrement", preference: "hotkey_decrement", code: STANDARD | 60, keysym: 0xffbf },
    BotAction { name: "reset", preference: "hotkey_reset", code: STANDARD | 61, keysym: 0xffc0 },
    BotAction { name: "undo", preference: "hotkey_undo", code: STANDARD | 62, keysym: 0xffc1 },
    BotAction { name: "redo", preference: "hotkey_redo", code: STANDARD | 63, keysym: 0xffc2 },
    BotAction { name: "minimize", preference: "hotkey_minimize", code: STANDARD | 64, keysym: 0xffc3 },
    BotAction { name: "alt-std", preference: "hotkey_alt_std", code: STANDARD | 65, keysym: 0xffc4 },
    BotAction { name: "lock", preference: "hotkey_lock", code: STANDARD | 66, keysym: 0xffc5 },
    BotAction { name: "boat", preference: "hotkey_boat", code: STANDARD | 67, keysym: 0xffc6 },
    BotAction { name: "mod-360", preference: "hotkey_mod_360", code: STANDARD | 68, keysym: 0xffc7 },
    BotAction { name: "aa-mode", preference: "hotkey_toggle_aa_mode", code: STANDARD | 87, keysym: 0xffc8 },
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    /// Trigger one of the bot's actions.
    Bot(&'static BotAction),
    Show,
    Hide,
    Toggle,
    ShowThrows,
    HideThrows,
    ToggleThrows,
    /// Reload the configuration from disk.
    Reload,
    /// Stop: the overlay, the box, and the bot inside it.
    Quit,
}

impl Action {
    pub fn parse(name: &str) -> Option<Action> {
        Some(match name {
            "show" => Action::Show,
            "hide" => Action::Hide,
            "toggle" => Action::Toggle,
            "show-throws" => Action::ShowThrows,
            "hide-throws" => Action::HideThrows,
            "toggle-throws" => Action::ToggleThrows,
            "reload" => Action::Reload,
            "quit" => Action::Quit,
            _ => Action::Bot(BOT_ACTIONS.iter().find(|action| action.name == name)?),
        })
    }

    pub fn names() -> Vec<&'static str> {
        let mut names = vec![
            "show",
            "hide",
            "toggle",
            "show-throws",
            "hide-throws",
            "toggle-throws",
            "reload",
            "quit",
        ];
        names.extend(BOT_ACTIONS.iter().map(|action| action.name));
        names
    }
}
