use std::rc::Rc;

use jay_config::{
    ContainerTarget, Direction, RelativeAxis, get_workspace,
    input::{LayerDirection, Seat, Timeline, get_default_seat},
    keyboard::{
        ModifiedKeySym,
        mods::{ALT, CTRL, LOGO, SHIFT},
        syms::*,
    },
    reload, set_show_titles, switch_to_vt,
};

use crate::{bar, exec, idle};

/// key, arrow key, direction, and how resizing grows the window that way as
/// `(dx1, dy1, dx2, dy2)`
const DIRS: [(KeySym, KeySym, Direction, [i32; 4]); 4] = [
    (SYM_h, SYM_Left, Direction::Left, [-1, 0, 0, 0]),
    (SYM_j, SYM_Down, Direction::Down, [0, 0, 0, 1]),
    (SYM_k, SYM_Up, Direction::Up, [0, -1, 0, 0]),
    (SYM_l, SYM_Right, Direction::Right, [0, 0, 1, 0]),
];

const DIGITS: [KeySym; 10] = [
    SYM_0, SYM_1, SYM_2, SYM_3, SYM_4, SYM_5, SYM_6, SYM_7, SYM_8, SYM_9,
];

const VTS: [KeySym; 12] = [
    SYM_F1, SYM_F2, SYM_F3, SYM_F4, SYM_F5, SYM_F6, SYM_F7, SYM_F8, SYM_F9, SYM_F10, SYM_F11,
    SYM_F12,
];

const RESIZE_STEP: i32 = 10;

fn bind_repeating(seat: Seat, sym: impl Into<ModifiedKeySym>, f: impl FnMut() + 'static) {
    let sym = sym.into();
    seat.bind(sym, f);
    seat.set_repeat_bind(sym, true);
}

/// Shortcuts layered on top of the normal ones while the mode is active. jay's
/// config API has no mode stack, so entering binds them and exiting unbinds
/// them again; only one mode is ever active.
struct Mode {
    name: &'static str,
    key: ModifiedKeySym,
    shortcuts: Vec<(ModifiedKeySym, Shortcut)>,
}

enum Shortcut {
    /// exits the mode, then runs
    Exit(Rc<dyn Fn()>),
    /// stays in the mode and repeats while held
    Repeat(Rc<dyn Fn()>),
}

impl Mode {
    fn new(name: &'static str, key: impl Into<ModifiedKeySym>) -> Self {
        Self {
            name,
            key: key.into(),
            shortcuts: vec![],
        }
    }

    fn exit_with(mut self, sym: impl Into<ModifiedKeySym>, f: impl Fn() + 'static) -> Self {
        self.shortcuts
            .push((sym.into(), Shortcut::Exit(Rc::new(f))));
        self
    }

    fn repeat(mut self, sym: impl Into<ModifiedKeySym>, f: impl Fn() + 'static) -> Self {
        self.shortcuts
            .push((sym.into(), Shortcut::Repeat(Rc::new(f))));
        self
    }

    fn install(self, seat: Seat) {
        Rc::new(self).bind_entry(seat);
    }

    fn bind_entry(self: &Rc<Self>, seat: Seat) {
        let mode = self.clone();
        seat.bind(self.key, move || mode.enter(seat));
    }

    fn enter(self: &Rc<Self>, seat: Seat) {
        for (sym, shortcut) in &self.shortcuts {
            match shortcut {
                Shortcut::Exit(f) => {
                    let (mode, f) = (self.clone(), f.clone());
                    seat.bind(*sym, move || {
                        mode.exit(seat);
                        f();
                    });
                }
                Shortcut::Repeat(f) => {
                    let f = f.clone();
                    bind_repeating(seat, *sym, move || f());
                }
            }
        }
        for sym in [self.key, SYM_Escape.into()] {
            let mode = self.clone();
            seat.bind(sym, move || mode.exit(seat));
        }
        bar::set_mode(Some(self.name));
    }

    fn exit(self: &Rc<Self>, seat: Seat) {
        for (sym, _) in &self.shortcuts {
            seat.unbind(*sym);
        }
        seat.unbind(SYM_Escape);
        self.bind_entry(seat);
        bar::set_mode(None);
    }
}

fn modes(seat: Seat) {
    let resize = |grow: [i32; 4], step: i32| {
        let [dx1, dy1, dx2, dy2] = grow.map(|d| d * step);
        move || seat.resize(dx1, dy1, dx2, dy2)
    };
    let mut resize_mode = Mode::new("resize", LOGO | SYM_r);
    for (key, arrow, _, grow) in DIRS {
        for sym in [key, arrow] {
            resize_mode = resize_mode
                .repeat(sym, resize(grow, RESIZE_STEP))
                .repeat(SHIFT | sym, resize(grow, -RESIZE_STEP));
        }
    }
    resize_mode.install(seat);

    Mode::new("system", LOGO | SYM_p)
        .exit_with(SYM_l, || exec("swaylock", &["--daemonize"]))
        .exit_with(SYM_s, || exec("systemctl", &["poweroff"]))
        .exit_with(SYM_r, || exec("systemctl", &["reboot"]))
        .exit_with(SYM_h, idle::suspend)
        .exit_with(SYM_i, idle::toggle_inhibitor)
        .install(seat);
}

/// Captures a region and opens it in satty.
fn screenshot((x, y): (i32, i32), (width, height): (i32, i32)) {
    if width > 0 && height > 0 {
        exec(
            "sh",
            &[
                "-c",
                &format!(
                    "jay-screenshot --type ppm --file - region '{x},{y} {width}x{height}' \
                     | satty --filename -"
                ),
            ],
        );
    }
}

fn move_to_output(seat: Seat, direction: Direction) {
    let target = seat
        .get_workspace()
        .connector()
        .connector_in_direction(direction);
    if target.exists() {
        seat.move_to_output(target);
    }
}

pub fn setup() {
    let seat = get_default_seat();

    for (key, arrow, direction, _) in DIRS {
        for sym in [key, arrow] {
            bind_repeating(seat, LOGO | sym, move || seat.focus(direction));
            bind_repeating(seat, LOGO | SHIFT | sym, move || seat.move_(direction));
            bind_repeating(seat, LOGO | SHIFT | CTRL | sym, move || {
                move_to_output(seat, direction)
            });
        }
    }

    for (n, sym) in DIGITS.into_iter().enumerate() {
        let name = n.to_string();
        let workspace = move || get_workspace(&name);
        let show = workspace.clone();
        seat.bind(LOGO | sym, move || seat.show_workspace(show()));
        seat.bind(LOGO | SHIFT | sym, move || seat.set_workspace(workspace()));
    }

    for (n, sym) in (1..).zip(VTS) {
        seat.bind(CTRL | ALT | sym, move || switch_to_vt(n));
    }

    // stops the session's services before jay itself
    seat.bind(LOGO | SHIFT | SYM_q, || {
        exec("systemctl", &["--user", "stop", "jay.service"])
    });
    seat.bind(LOGO | SHIFT | SYM_r, reload);

    // windows
    seat.bind(LOGO | SYM_q, move || seat.close());
    seat.bind(LOGO | SYM_f, move || seat.toggle_fullscreen());
    seat.bind(LOGO | SYM_space, move || seat.toggle_floating());
    seat.bind(LOGO | SYM_n, move || {
        seat.toggle_container_mono(ContainerTarget::Auto)
    });
    seat.bind(LOGO | SYM_v, move || {
        seat.toggle_container_split(ContainerTarget::Auto)
    });
    seat.bind(LOGO | SYM_b, move || {
        seat.create_split_relative(RelativeAxis::Major)
    });
    seat.bind(LOGO | SYM_Escape, move || seat.disable_pointer_constraint());
    seat.bind(LOGO | SYM_t, || set_show_titles(true));
    seat.bind(LOGO | SHIFT | SYM_t, || set_show_titles(false));
    seat.bind(LOGO | SYM_y, move || {
        seat.set_container_split_relative(ContainerTarget::Parent, RelativeAxis::Major)
    });
    seat.bind(LOGO | SHIFT | SYM_y, move || {
        seat.create_split_relative(RelativeAxis::Major)
    });

    // mirror of the laptop panel
    seat.bind(LOGO | SYM_z, || exec("wl-present", &["freeze"]));
    seat.bind(LOGO | SHIFT | SYM_z, || exec("wl-present", &["unfreeze"]));

    // focus
    bind_repeating(seat, LOGO | SYM_Tab, move || {
        seat.focus_history(Timeline::Newer)
    });
    bind_repeating(seat, LOGO | SHIFT | SYM_Tab, move || {
        seat.focus_history(Timeline::Older)
    });
    bind_repeating(seat, LOGO | SYM_Prior, move || {
        seat.focus_layer_rel(LayerDirection::Above)
    });
    bind_repeating(seat, LOGO | SYM_Next, move || {
        seat.focus_layer_rel(LayerDirection::Below)
    });
    bind_repeating(seat, LOGO | SYM_g, move || seat.focus_parent());
    seat.bind(LOGO | SYM_Delete, move || seat.focus_tiles());
    seat.bind(LOGO | SYM_c, move || seat.warp_mouse_to_focus());

    modes(seat);

    // media
    bind_repeating(seat, SYM_XF86AudioRaiseVolume, || {
        exec(
            "wpctl",
            &[
                "set-volume",
                "@DEFAULT_AUDIO_SINK@",
                "5%+",
                "--limit",
                "1.5",
            ],
        )
    });
    bind_repeating(seat, SYM_XF86AudioLowerVolume, || {
        exec(
            "wpctl",
            &[
                "set-volume",
                "@DEFAULT_AUDIO_SINK@",
                "5%-",
                "--limit",
                "0.0",
            ],
        )
    });
    seat.bind(SYM_XF86AudioMute, || {
        exec("wpctl", &["set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"])
    });
    seat.bind(SYM_XF86AudioMicMute, || {
        exec("wpctl", &["set-mute", "@DEFAULT_AUDIO_SOURCE@", "toggle"])
    });
    seat.bind(SYM_XF86AudioPlay, || exec("playerctl", &["play-pause"]));
    seat.bind(SYM_XF86AudioPause, || exec("playerctl", &["play-pause"]));
    seat.bind(SYM_XF86AudioNext, || exec("playerctl", &["next"]));
    seat.bind(SYM_XF86AudioPrev, || exec("playerctl", &["previous"]));
    seat.bind(SYM_XF86AudioStop, || exec("playerctl", &["stop"]));

    // screenshots
    seat.bind(LOGO | SYM_s, move || {
        let output = seat.get_keyboard_connector();
        screenshot(output.position(), output.size());
    });
    seat.bind(LOGO | SHIFT | SYM_s, move || {
        let window = seat.window();
        screenshot(window.position(), window.size());
    });
    seat.bind(LOGO | CTRL | SYM_s, move || {
        let workspace = seat.get_workspace();
        screenshot(workspace.position(), workspace.size());
    });

    // launchers
    seat.bind(LOGO | SYM_Return, || exec("runapp", &["footclient"]));
    seat.bind(LOGO | SHIFT | SYM_Return, || exec("runapp", &["foot"]));
    seat.bind(LOGO | SYM_d, || exec("fuzzel", &[]));
    seat.bind(LOGO | SYM_a, || exec("swaync-client", &["-t"]));
    seat.bind(LOGO | SHIFT | SYM_v, || {
        exec(
            "sh",
            &[
                "-c",
                "cliphist list | fuzzel --dmenu --with-nth 2 | cliphist decode | wl-copy",
            ],
        )
    });
}
