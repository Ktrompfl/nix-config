{ ... }:
{
  # The keyboards, the pointer, and the lid switch.
  wayland.windowManager.jay.settings = {
    # see https://wiki.archlinux.org/title/X_keyboard_extension for xkb keymap settings
    keymaps = [
      {
        name = "laptop";
        map = ''
          xkb_keymap {
            xkb_keycodes { include "evdev+aliases(qwertz)" };
            xkb_types    { include "complete" };
            xkb_compat   { include "complete" };
            xkb_symbols  { include "pc+de(qwerty)+inet(evdev)+capslock(escape)" };
            xkb_geometry { include "pc(pc105)" };
          };
        '';
      }
      {
        name = "external";
        map = ''
          xkb_keymap {
            xkb_keycodes { include "evdev+aliases(qwerty)" };
            xkb_types    { include "complete" };
            xkb_compat   { include "complete" };
            xkb_symbols  { include "pc+us(altgr-intl)+inet(evdev)+capslock(escape)" };
            xkb_geometry { include "pc(pc104)"};
          };
        '';
      }
    ];

    # find names of inputs with 'jay input'
    inputs = [
      {
        tag = "pointer";
        match.is-pointer = true;
        accel-profile = "Flat";
        accel-speed = 0;
        left-handed = false;
        tap-enabled = true;
        natural-scrolling = false;
      }
      {
        tag = "laptop-keyboard";
        match.name = "AT Translated Set 2 keyboard";
        keymap.name = "laptop";
      }
      {
        # all input devices exposed by the Yunzii/SmartCloud AL68 keyboard;
        # `match` has no regex form, so they are spelled out one by one
        tag = "external-keyboard";
        match = map (name: { inherit name; }) [
          "I-CHIP YUNZII AL68 2.4G"
          "I-CHIP YUNZII AL68 2.4G Keyboard"
          "I-CHIP YUNZII AL68 2.4G Mouse"
          "SmartCloud AL68 Keyboard"
          "SmartCloud AL68 Keyboard Mouse"
          "SmartCloud AL68 Keyboard Consumer Control"
          "SmartCloud AL68 Keyboard System Control"
        ];
        keymap.name = "external";
      }
      {
        match.name = "Lid Switch";
        on-lid-closed = {
          type = "configure-connector";
          connector = {
            match.name = "laptop-integrated";
            enabled = false;
          };
        };
        on-lid-opened = {
          type = "configure-connector";
          connector = {
            match.name = "laptop-integrated";
            enabled = true;
          };
        };
      }
    ];
  };
}
