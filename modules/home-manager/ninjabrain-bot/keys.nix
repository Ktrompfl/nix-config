# What the bot stores a hotkey as: `code | location << 16`, where for the main
# key block `code` is the evdev code, which is the X keycode minus 8. Measured:
# injecting keycodes 27, 14, 47, 65 and 69 made the bot's recorder store 65555,
# 65542, 65575, 65593 and 65597.
#
# Extended (0xE0-prefixed) keys are absent because they break that rule: the
# bot stores numpad 5 as `4 << 16 | 0xE04C` and will not record an arrow at
# all. Set `code` and `location` by hand for those -- but anything replaying a
# hotkey has to undo the same arithmetic, so it will not be able to send them.
{
  keys = {
    escape = 1;

    "1" = 2;
    "2" = 3;
    "3" = 4;
    "4" = 5;
    "5" = 6;
    "6" = 7;
    "7" = 8;
    "8" = 9;
    "9" = 10;
    "0" = 11;
    minus = 12;
    equal = 13;
    backspace = 14;
    tab = 15;

    q = 16;
    w = 17;
    e = 18;
    r = 19;
    t = 20;
    y = 21;
    u = 22;
    i = 23;
    o = 24;
    p = 25;
    bracketleft = 26;
    bracketright = 27;
    enter = 28;

    a = 30;
    s = 31;
    d = 32;
    f = 33;
    g = 34;
    h = 35;
    j = 36;
    k = 37;
    l = 38;
    semicolon = 39;
    apostrophe = 40;
    grave = 41;
    backslash = 43;

    z = 44;
    x = 45;
    c = 46;
    v = 47;
    b = 48;
    n = 49;
    m = 50;
    comma = 51;
    period = 52;
    slash = 53;

    space = 57;
    capslock = 58;

    F1 = 59;
    F2 = 60;
    F3 = 61;
    F4 = 62;
    F5 = 63;
    F6 = 64;
    F7 = 65;
    F8 = 66;
    F9 = 67;
    F10 = 68;
    F11 = 87;
    F12 = 88;
  };

  # The bot matches modifiers as a bit mask. Masks from JNativeHook.
  modifiers = {
    SHIFT_L = 1;
    CTRL_L = 2;
    META_L = 4;
    ALT_L = 8;
    SHIFT_R = 16;
    CTRL_R = 32;
    META_R = 64;
    ALT_R = 128;
  };

  # The `location` half of a code. Everything above is STANDARD.
  locations = {
    UNKNOWN = 0;
    STANDARD = 1;
    LEFT = 2;
    RIGHT = 3;
    NUMPAD = 4;
  };

  # Action name on the command line, and the preference it is stored under.
  actions = {
    increment = "hotkey_increment";
    decrement = "hotkey_decrement";
    reset = "hotkey_reset";
    undo = "hotkey_undo";
    redo = "hotkey_redo";
    minimize = "hotkey_minimize";
    alt-std = "hotkey_alt_std";
    lock = "hotkey_lock";
    boat = "hotkey_boat";
    mod-360 = "hotkey_mod_360";
    aa-mode = "hotkey_toggle_aa_mode";
  };
}
