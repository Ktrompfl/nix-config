use std::{
    cell::RefCell,
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
};

use jay_config::exec::Command;
use serde_json::Value;

use super::exec::capture_lines;

type Subscriber = Box<dyn Fn(&Value)>;

thread_local! {
    static SUBSCRIBERS: RefCell<HashMap<u32, Subscriber>> = RefCell::new(HashMap::new());
    // i3status-rs re-emits every block's full_text on *any* block's change
    // (see `dispatch`'s doc comment), so most blocks are unchanged on most
    // ticks; skipping the second JSON parse and the subscriber (which would
    // just reformat identical data and re-push an identical status string)
    // for those needs comparing against what each block last rendered.
    static LAST_TEXT_HASH: RefCell<HashMap<u32, u64>> = RefCell::new(HashMap::new());
}

fn hash(text: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

/// Registers `on_update` to be called with block `id`'s data every time
/// `i3status-rs` prints a new status line. `id` is the block's 0-based
/// position in `config-jay.toml`'s `block` list, which `i3status-rs` echoes
/// back as the `"{id}:"` prefix of each block's `instance` field regardless
/// of which blocks are currently hidden.
///
/// `on_update` gets the block's *entire* data as a JSON object (each
/// `config-jay.toml` block's `format` renders every non-icon placeholder it
/// has, not just the ones used today) - picking which fields to use,
/// deciding severity thresholds/colors, and choosing icons is entirely this
/// module's callers' job, not `i3status-rs`'s.
pub fn subscribe(id: u32, on_update: impl Fn(&Value) + 'static) {
    SUBSCRIBERS.with(|s| {
        s.borrow_mut().insert(id, Box::new(on_update));
    });
}

fn block_id(instance: &str) -> Option<u32> {
    instance.split_once(':')?.0.parse().ok()
}

/// Reads `key` out of a block's data object as a number. i3status-rs
/// renders every numeric placeholder as a plain, unit-less string (see
/// i3status-rust.nix's `numFmt`), and JSON `null` (used for placeholders
/// that were absent) fails `as_str`, so this doubles as an absence check.
pub fn number(data: &Value, key: &str) -> Option<f64> {
    data.get(key)?.as_str()?.parse().ok()
}

/// Reads `key` out of a block's data object as text.
pub fn text<'a>(data: &'a Value, key: &str) -> Option<&'a str> {
    data.get(key)?.as_str()
}

fn dispatch(line: &str) {
    // Every status update is a JSON array of blocks followed by a literal
    // trailing comma (see i3status-rs's `protocol::print_blocks`); the two
    // header lines it prints once at startup (`{"version":...}` and a lone
    // `[`) don't parse as that shape, so they're simply skipped here rather
    // than special-cased.
    let Ok(Value::Array(blocks)) = serde_json::from_str::<Value>(line.trim_end().trim_end_matches(',')) else {
        return;
    };

    // i3status-rs splits a single block's rendered text across *multiple*
    // array entries sharing the same `instance` whenever its format string
    // has a conditional group (`{a|b}`): each placeholder that needs its
    // own click-routing gets its own i3bar segment, so e.g. cpu's
    // `\{"utilization":"1.9","frequency":{...|null},"max_frequency":{...|null}\}`
    // arrives as three separate entries, each holding a fragment. The full
    // per-block text - and therefore valid JSON again - only exists after
    // concatenating every entry with a given id, in array order.
    let mut texts: HashMap<u32, String> = HashMap::new();
    for block in &blocks {
        let Some(id) = block.get("instance").and_then(|v| v.as_str()).and_then(block_id) else {
            continue;
        };
        let Some(fragment) = block.get("full_text").and_then(|v| v.as_str()) else {
            continue;
        };
        texts.entry(id).or_default().push_str(fragment);
    }

    for (id, text) in texts {
        let text_hash = hash(&text);
        let unchanged = LAST_TEXT_HASH.with(|h| {
            let mut h = h.borrow_mut();
            let unchanged = h.get(&id) == Some(&text_hash);
            h.insert(id, text_hash);
            unchanged
        });
        if unchanged {
            continue;
        }

        // `text` is itself the JSON object `config-jay.toml` rendered as
        // text (see i3status-rust.nix's `obj`/`tagged` helpers), so it
        // needs a second parse.
        let Ok(data) = serde_json::from_str::<Value>(&text) else {
            continue;
        };
        SUBSCRIBERS.with(|s| {
            if let Some(on_update) = s.borrow().get(&id) {
                on_update(&data);
            }
        });
    }
}

/// Spawns the long-lived `i3status-rs` process that backs every other
/// "system data" block in this module (cpu/memory/disk/backlight/battery/
/// network/audio/notifications): their `run()` functions only
/// `subscribe()` to the block index they care about instead of reading
/// sysfs/proc or talking to wpctl/pipewire/dbus themselves.
pub fn run() {
    capture_lines(Command::new("i3status-rs").arg("config-jay.toml"), |line| dispatch(&line));
}
