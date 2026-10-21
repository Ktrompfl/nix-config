use jay_config::{
    input::{
        SwitchEvent, acceleration::ACCEL_PROFILE_FLAT, capability::CAP_POINTER, on_new_input_device,
    },
    keyboard::parse_keymap,
};

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
    let external_keymap = parse_keymap(EXTERNAL_MAP);

    on_new_input_device(move |device| {
        if device.has_capability(CAP_POINTER) {
            device.set_accel_profile(ACCEL_PROFILE_FLAT);
            device.set_accel_speed(0.0);
            device.set_left_handed(false);
            device.set_tap_enabled(true);
            device.set_natural_scrolling_enabled(false);
        }

        let name = device.name();
        if name == "AT Translated Set 2 keyboard" {
            device.set_keymap(laptop_keymap);
        } else if name.contains("AL68") {
            device.set_keymap(external_keymap);
        } else if name == "Lid Switch" {
            device.on_switch_event(|event| {
                let panel = outputs::LAPTOP_PANEL.connector();
                match event {
                    SwitchEvent::LidClosed => panel.set_enabled(false),
                    SwitchEvent::LidOpened => panel.set_enabled(true),
                    _ => {}
                }
            });
        }
    });
}
