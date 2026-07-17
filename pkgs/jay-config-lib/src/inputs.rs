use jay_config::{
    exec::Command,
    input::{SwitchEvent, acceleration::ACCEL_PROFILE_FLAT, capability::CAP_POINTER, on_new_input_device},
    keyboard::{Keymap, parse_keymap},
};
use regex::Regex;

use crate::outputs;

const LAPTOP_MAP: &str = r#"
xkb_keymap {
  xkb_keycodes { include "evdev+aliases(qwertz)" };
  xkb_types    { include "complete" };
  xkb_compat   { include "complete" };
  xkb_symbols  { include "pc+de(qwerty)+inet(evdev)+capslock(escape)" };
  xkb_geometry { include "pc(pc105)" };
};
"#;

const EXTERNAL_MAP: &str = r#"
xkb_keymap {
  xkb_keycodes { include "evdev+aliases(qwerty)" };
  xkb_types    { include "complete" };
  xkb_compat   { include "complete" };
  xkb_symbols  { include "pc+us(altgr-intl)+inet(evdev)+capslock(escape)" };
  xkb_geometry { include "pc(pc104)"};
};
"#;

pub fn setup() {
    let laptop_keymap = parse_keymap(LAPTOP_MAP);
    let external_keymap: Keymap = parse_keymap(EXTERNAL_MAP);
    // matches all the input devices exposed by the Yunzii/SmartCloud AL68 keyboard
    let external_keyboard = Regex::new("AL68").unwrap();

    on_new_input_device(move |dev| {
        if dev.has_capability(CAP_POINTER) {
            dev.set_accel_profile(ACCEL_PROFILE_FLAT);
            dev.set_accel_speed(0.0);
            dev.set_left_handed(false);
            dev.set_tap_enabled(true);
            dev.set_natural_scrolling_enabled(false);
        }

        let name = dev.name();

        if name == "AT Translated Set 2 keyboard" {
            dev.set_keymap(laptop_keymap);
        } else if external_keyboard.is_match(&name) {
            dev.set_keymap(external_keymap);
        } else if name == "Lid Switch" {
            dev.on_switch_event(|event| {
                let laptop_screen = outputs::laptop_integrated();
                match event {
                    SwitchEvent::LidClosed => {
                        log::info!("lid closed: disabling internal display");
                        laptop_screen.set_enabled(false);
                    }
                    SwitchEvent::LidOpened => {
                        log::info!("lid opened: re-enabling internal display");
                        laptop_screen.set_enabled(true);
                    }
                    _ => {}
                }
            });
        }
    });
}
