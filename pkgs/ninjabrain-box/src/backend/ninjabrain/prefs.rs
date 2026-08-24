//! Writing the bot's `java.util.prefs` file.
//!
//! The bot has no configuration format of its own: it keeps everything in a
//! `prefs.xml` of flat key/value entries that its GUI writes. Since the
//! overlay owns the instance it starts, it writes that file too, and the
//! settings that survive are the ones that change the arithmetic -- the rest
//! describe a window nobody sees.

use anyhow::{Context, Result};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use super::hotkeys::HOTKEYS;
use crate::config::Settings;

/// Where java.util.prefs looks, under the root handed to the JVM.
pub fn file(root: &Path) -> PathBuf {
    root.join(".java/.userPrefs/ninjabrainbot/prefs.xml")
}

/// Writes the preferences for the bot instance the overlay is about to start.
pub fn write(root: &Path, settings: &Settings) -> Result<()> {
    let path = file(root);
    let directory = path.parent().expect("the preferences path has a parent");
    std::fs::create_dir_all(directory)
        .with_context(|| format!("cannot create {}", directory.display()))?;
    std::fs::write(&path, document(settings))
        .with_context(|| format!("cannot write {}", path.display()))
}

fn document(settings: &Settings) -> String {
    let mut entries: Vec<(String, String)> = Vec::new();
    let mut put = |key: &str, value: String| entries.push((key.to_owned(), value));

    // What the overlay needs to be true, whatever the configuration says.
    //
    // The HTTP API is where the panel comes from. The clipboard is polled
    // rather than read on a hotkey, because polling is what notices the
    // selection the overlay put on the box's display. And the settings
    // version has to be current, or the bot rewrites every hotkey code on
    // startup, assuming they came from an older release.
    put("settings_version", "3".into());
    put("enable_http_server", "true".into());
    put("alt_clipboard_reader", "false".into());
    put("check_for_updates", "false".into());
    put("use_obs_overlay", "false".into());

    // Every action is bound to a function key. Nothing outside the box can
    // press one -- it is a display with one client and no input devices -- so
    // these never collide with anything the game or the compositor binds.
    for hotkey in HOTKEYS {
        put(&format!("{}_code", hotkey.preference), hotkey.code.to_string());
        put(&format!("{}_modifier", hotkey.preference), "0".into());
    }

    for (key, value) in settings.entries() {
        put(&key, value);
    }

    // Sorted, so the file only changes when the configuration does.
    entries.sort();
    let mut document = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\"?>\n\
         <!DOCTYPE map SYSTEM \"http://java.sun.com/dtd/preferences.dtd\">\n\
         <map MAP_XML_VERSION=\"1.0\">\n",
    );
    for (key, value) in entries {
        let _ = writeln!(
            document,
            "  <entry key=\"{}\" value=\"{}\"/>",
            escape(&key),
            escape(&value)
        );
    }
    document.push_str("</map>\n");
    document
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
