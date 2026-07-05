use jay_config::{
    Axis, Direction, get_workspace,
    exec::Command,
    input::{Seat, get_default_seat},
    keyboard::{
        mods::{ALT, CTRL, LOGO, SHIFT},
        syms::*,
    },
    quit, reload, set_show_titles,
    video::Connector,
};

use crate::modes;

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

// FIXME: right now there is no way to get the region of the focused window / workspace
fn screenshot(connector: Connector) {
    if !connector.exists() {
        return;
    }
    let name = connector.name();
    Command::new("sh")
        .arg("-c")
        .arg(&format!("grim -o '{name}' - | satty --filename -"))
        .spawn();
}

fn screenshot_output(seat: Seat) {
    screenshot(seat.get_keyboard_connector());
}

fn screenshot_window(seat: Seat) {
    screenshot(seat.window().workspace().connector());
}

fn screenshot_workspace(seat: Seat) {
    screenshot(seat.get_workspace().connector());
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
    seat.bind(LOGO | SYM_m, move || modes::push_mirror(seat));
    seat.bind(LOGO | SYM_p, move || modes::push_system(seat));
    seat.bind(LOGO | SYM_r, move || modes::push_resize(seat));

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
