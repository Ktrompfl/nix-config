use jay_config::{
    Axis, Direction, get_workspace,
    exec::Command,
    input::{Seat, get_default_seat},
    keyboard::{
        mods::{ALT, CTRL, LOGO, SHIFT},
        syms::*,
    },
    quit, reload, set_show_titles,
};

struct DirKey {
    key: jay_config::keyboard::syms::KeySym,
    arrow: jay_config::keyboard::syms::KeySym,
    dir: Direction,
}

const DIR_KEYS: [DirKey; 4] = [
    DirKey { key: SYM_h, arrow: SYM_Left, dir: Direction::Left },
    DirKey { key: SYM_j, arrow: SYM_Down, dir: Direction::Down },
    DirKey { key: SYM_k, arrow: SYM_Up, dir: Direction::Up },
    DirKey { key: SYM_l, arrow: SYM_Right, dir: Direction::Right },
];

fn move_output_direction(seat: Seat, direction: Direction) {
    let ws = seat.get_workspace();
    if !ws.exists() {
        return;
    }
    let source = ws.connector();
    if !source.exists() {
        return;
    }
    let target = source.connector_in_direction(direction);
    if !target.exists() {
        return;
    }
    seat.move_to_output(target);
}

fn shell(script: &'static str) {
    Command::new("sh").arg("-c").arg(script).spawn();
}

fn screenshot_region(x: i32, y: i32, w: i32, h: i32) {
    if w <= 0 || h <= 0 {
        return;
    }
    let geometry = format!("{x},{y} {w}x{h}");
    Command::new("sh")
        .arg("-c")
        .arg(&format!("grim -g '{geometry}' - | satty --filename -"))
        .spawn();
}

fn screenshot_output(seat: Seat) {
    let connector = seat.get_keyboard_connector();
    if !connector.exists() {
        return;
    }
    let (x, y) = connector.position();
    let (w, h) = connector.size();
    screenshot_region(x, y, w, h);
}

fn screenshot_window(seat: Seat) {
    let window = seat.window();
    if !window.exists() {
        return;
    }
    let (x, y) = window.position();
    let (w, h) = window.size();
    screenshot_region(x, y, w, h);
}

fn screenshot_workspace(seat: Seat) {
    let ws = seat.get_workspace();
    if !ws.exists() {
        return;
    }
    let (x, y) = ws.position();
    let (w, h) = ws.size();
    screenshot_region(x, y, w, h);
}

// --- modes ---
//
// Only one mode is ever active on top of normal mode: entering a mode
// rebinds its toggle key (e.g. LOGO+m) to pop it instead of pushing it, and
// popping restores just that one binding. Normal mode's own bindings are
// never torn down, so there's no "empty" state to rebuild from.

fn notify_push(name: &str) {
    crate::bar::set_mode(Some(name));
}

fn notify_pop() {
    crate::bar::set_mode(None);
}

fn push_mirror(seat: Seat) {
    seat.bind(LOGO | SYM_m, move || pop_mirror(seat));
    seat.bind(SYM_Escape, move || pop_mirror(seat));

    seat.bind(SYM_m, move || present(seat, "mirror"));
    seat.bind(SYM_c, move || present(seat, "custom"));
    seat.bind(SYM_f, move || present(seat, "toggle-freeze"));
    seat.bind(SYM_z, move || present(seat, "freeze"));
    seat.bind(SHIFT | SYM_z, move || present(seat, "unfreeze"));
    seat.bind(SYM_o, move || present(seat, "set-output"));
    seat.bind(SYM_r, move || present(seat, "set-region"));
    seat.bind(SHIFT | SYM_r, move || present(seat, "unset-region"));
    seat.bind(SYM_s, move || present(seat, "set-scaling"));

    notify_push("mirror");
}

fn present(seat: Seat, action: &'static str) {
    pop_mirror(seat);
    Command::new("wl-present").arg(action).spawn();
}

fn pop_mirror(seat: Seat) {
    seat.unbind(SYM_Escape);
    seat.unbind(SYM_m);
    seat.unbind(SYM_c);
    seat.unbind(SYM_f);
    seat.unbind(SYM_z);
    seat.unbind(SHIFT | SYM_z);
    seat.unbind(SYM_o);
    seat.unbind(SYM_r);
    seat.unbind(SHIFT | SYM_r);
    seat.unbind(SYM_s);
    seat.bind(LOGO | SYM_m, move || push_mirror(seat));
    notify_pop();
}

#[derive(Clone, Copy)]
enum ResizeField {
    Dx1,
    Dy1,
    Dx2,
    Dy2,
}

#[derive(Clone, Copy)]
struct ResizeKey {
    key: jay_config::keyboard::syms::KeySym,
    arrow: jay_config::keyboard::syms::KeySym,
    field: ResizeField,
    sign: i32,
}

const RESIZE_KEYS: [ResizeKey; 4] = [
    ResizeKey { key: SYM_h, arrow: SYM_Left, field: ResizeField::Dx1, sign: -1 },
    ResizeKey { key: SYM_j, arrow: SYM_Down, field: ResizeField::Dy2, sign: 1 },
    ResizeKey { key: SYM_k, arrow: SYM_Up, field: ResizeField::Dy1, sign: -1 },
    ResizeKey { key: SYM_l, arrow: SYM_Right, field: ResizeField::Dx2, sign: 1 },
];

const RESIZE_AMOUNT: i32 = 10;

fn resize_delta(field: ResizeField, val: i32) -> (i32, i32, i32, i32) {
    match field {
        ResizeField::Dx1 => (val, 0, 0, 0),
        ResizeField::Dy1 => (0, val, 0, 0),
        ResizeField::Dx2 => (0, 0, val, 0),
        ResizeField::Dy2 => (0, 0, 0, val),
    }
}

fn push_resize(seat: Seat) {
    for rk in RESIZE_KEYS {
        let val = rk.sign * RESIZE_AMOUNT;

        let (dx1, dy1, dx2, dy2) = resize_delta(rk.field, val);
        seat.bind(rk.key, move || seat.resize(dx1, dy1, dx2, dy2));
        seat.set_repeat_bind(rk.key, true);
        seat.bind(rk.arrow, move || seat.resize(dx1, dy1, dx2, dy2));
        seat.set_repeat_bind(rk.arrow, true);

        let (dx1, dy1, dx2, dy2) = resize_delta(rk.field, -val);
        seat.bind(SHIFT | rk.key, move || seat.resize(dx1, dy1, dx2, dy2));
        seat.set_repeat_bind(SHIFT | rk.key, true);
        seat.bind(SHIFT | rk.arrow, move || seat.resize(dx1, dy1, dx2, dy2));
        seat.set_repeat_bind(SHIFT | rk.arrow, true);
    }

    seat.bind(LOGO | SYM_r, move || pop_resize(seat));
    seat.bind(SYM_Escape, move || pop_resize(seat));

    notify_push("resize");
}

fn pop_resize(seat: Seat) {
    for rk in RESIZE_KEYS {
        seat.unbind(rk.key);
        seat.unbind(rk.arrow);
        seat.unbind(SHIFT | rk.key);
        seat.unbind(SHIFT | rk.arrow);
    }
    seat.unbind(SYM_Escape);
    seat.bind(LOGO | SYM_r, move || push_resize(seat));
    notify_pop();
}

fn push_system(seat: Seat) {
    seat.bind(LOGO | SYM_p, move || pop_system(seat));
    seat.bind(SYM_Escape, move || pop_system(seat));
    seat.bind(SYM_l, move || {
        pop_system(seat);
        Command::new("swaylock").arg("--daemonize").spawn();
    });
    seat.bind(SYM_s, move || {
        pop_system(seat);
        Command::new("systemctl").arg("poweroff").spawn();
    });
    seat.bind(SYM_r, move || {
        pop_system(seat);
        Command::new("systemctl").arg("reboot").spawn();
    });
    seat.bind(SYM_h, move || {
        pop_system(seat);
        Command::new("systemctl").arg("suspend").spawn();
    });

    notify_push("system");
}

fn pop_system(seat: Seat) {
    seat.unbind(SYM_Escape);
    seat.unbind(SYM_l);
    seat.unbind(SYM_s);
    seat.unbind(SYM_r);
    seat.unbind(SYM_h);
    seat.bind(LOGO | SYM_p, move || push_system(seat));
    notify_pop();
}

pub fn setup() {
    let seat = get_default_seat();

    // --- directional focus / move / move-to-output bindings ---
    for dk in DIR_KEYS {
        let dir = dk.dir;
        seat.bind(LOGO | dk.key, move || seat.focus(dir));
        seat.bind(LOGO | dk.arrow, move || seat.focus(dir));

        seat.bind(LOGO | SHIFT | dk.key, move || seat.move_(dir));
        seat.bind(LOGO | SHIFT | dk.arrow, move || seat.move_(dir));

        seat.bind(LOGO | SHIFT | CTRL | dk.key, move || move_output_direction(seat, dir));
        seat.bind(LOGO | SHIFT | CTRL | dk.arrow, move || move_output_direction(seat, dir));
    }

    // --- workspace bindings ---
    const WORKSPACE_SYMS: [jay_config::keyboard::syms::KeySym; 10] =
        [SYM_0, SYM_1, SYM_2, SYM_3, SYM_4, SYM_5, SYM_6, SYM_7, SYM_8, SYM_9];
    const WORKSPACE_NAMES: [&str; 10] = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"];
    for (sym, name) in WORKSPACE_SYMS.into_iter().zip(WORKSPACE_NAMES) {
        seat.bind(LOGO | sym, move || {
            seat.show_workspace(get_workspace(name));
        });
        seat.bind(LOGO | SHIFT | sym, move || {
            seat.set_workspace(get_workspace(name));
        });
    }

    // --- switch to VT ---
    const VT_SYMS: [jay_config::keyboard::syms::KeySym; 12] = [
        SYM_F1, SYM_F2, SYM_F3, SYM_F4, SYM_F5, SYM_F6, SYM_F7, SYM_F8, SYM_F9, SYM_F10, SYM_F11, SYM_F12,
    ];
    for (i, sym) in VT_SYMS.into_iter().enumerate() {
        let n = i as u32 + 1;
        seat.bind(CTRL | ALT | sym, move || jay_config::switch_to_vt(n));
    }

    // --- compositor ---
    seat.bind(LOGO | SHIFT | SYM_q, quit);
    seat.bind(LOGO | SHIFT | SYM_r, reload);

    // --- windows ---
    seat.bind(LOGO | SYM_q, move || seat.close());
    seat.bind(LOGO | SYM_f, move || seat.toggle_fullscreen());
    seat.bind(LOGO | SYM_space, move || seat.toggle_floating());
    seat.bind(LOGO | SYM_n, move || seat.toggle_mono());
    seat.bind(LOGO | SYM_v, move || seat.toggle_split());
    seat.bind(LOGO | SYM_u, move || seat.create_split(Axis::Horizontal));
    seat.bind(LOGO | SYM_i, move || seat.create_split(Axis::Vertical));
    seat.bind(LOGO | SYM_Escape, move || seat.disable_pointer_constraint());
    seat.bind(LOGO | SYM_t, || set_show_titles(true));
    seat.bind(LOGO | SHIFT | SYM_t, || set_show_titles(false));

    // --- focus ---
    seat.bind(LOGO | SYM_Tab, move || seat.focus_history(jay_config::input::Timeline::Newer));
    seat.bind(LOGO | SHIFT | SYM_Tab, move || seat.focus_history(jay_config::input::Timeline::Older));
    seat.bind(LOGO | SYM_Delete, move || seat.focus_tiles());
    seat.bind(LOGO | SYM_Prior, move || seat.focus_layer_rel(jay_config::input::LayerDirection::Above));
    seat.bind(LOGO | SYM_Next, move || seat.focus_layer_rel(jay_config::input::LayerDirection::Below));
    seat.bind(LOGO | SYM_g, move || seat.focus_parent());
    seat.bind(LOGO | SYM_c, move || seat.warp_mouse_to_focus());

    // --- marks: the next key press identifies the mark ---
    seat.bind(LOGO | SYM_y, move || seat.jump_to_mark(None));
    seat.bind(LOGO | SHIFT | SYM_y, move || seat.create_mark(None));

    // --- modes ---
    seat.bind(LOGO | SYM_m, move || push_mirror(seat));
    seat.bind(LOGO | SYM_p, move || push_system(seat));
    seat.bind(LOGO | SYM_r, move || push_resize(seat));

    // --- audio (wireplumber) ---
    seat.bind(SYM_XF86AudioRaiseVolume, || {
        Command::new("wpctl")
            .arg("set-volume")
            .arg("@DEFAULT_AUDIO_SINK@")
            .arg("5%+")
            .arg("--limit")
            .arg("1.5")
            .spawn();
    });
    seat.bind(SYM_XF86AudioLowerVolume, || {
        Command::new("wpctl")
            .arg("set-volume")
            .arg("@DEFAULT_AUDIO_SINK@")
            .arg("5%-")
            .arg("--limit")
            .arg("0.0")
            .spawn();
    });
    seat.bind(SYM_XF86AudioMute, || {
        Command::new("wpctl").arg("set-mute").arg("@DEFAULT_AUDIO_SINK@").arg("toggle").spawn();
    });
    seat.bind(SYM_XF86AudioMicMute, || {
        Command::new("wpctl").arg("set-mute").arg("@DEFAULT_AUDIO_SOURCE@").arg("toggle").spawn();
    });

    // --- player ---
    seat.bind(SYM_XF86AudioPlay, || {
        Command::new("playerctl").arg("play-pause").spawn();
    });
    seat.bind(SYM_XF86AudioPause, || {
        Command::new("playerctl").arg("play-pause").spawn();
    });
    seat.bind(SYM_XF86AudioNext, || {
        Command::new("playerctl").arg("next").spawn();
    });
    seat.bind(SYM_XF86AudioPrev, || {
        Command::new("playerctl").arg("previous").spawn();
    });
    seat.bind(SYM_XF86AudioStop, || {
        Command::new("playerctl").arg("stop").spawn();
    });

    // --- screenshot ---
    seat.bind(LOGO | SYM_s, move || screenshot_output(seat));
    seat.bind(LOGO | SHIFT | SYM_s, move || screenshot_window(seat));
    seat.bind(LOGO | CTRL | SYM_s, move || screenshot_workspace(seat));

    // --- launch ---
    seat.bind(LOGO | SYM_Return, || {
        Command::new("app2unit").arg("footclient").spawn();
    });
    seat.bind(LOGO | SYM_d, || {
        Command::new("fuzzel").spawn();
    });
    seat.bind(LOGO | SYM_a, || {
        Command::new("swaync-client").arg("-t").spawn();
    });
    seat.bind(LOGO | SHIFT | SYM_v, || {
        shell("cliphist list | fuzzel --dmenu --with-nth 2 | cliphist decode | wl-copy");
    });
}
