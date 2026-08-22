//! Everything that is bound to a key, including the input modes.

use std::rc::Rc;

use jay_config::{
    get_workspace,
    input::{get_default_seat, LayerDirection, Seat, Timeline},
    keyboard::{
        mods::{ALT, CTRL, LOGO, SHIFT},
        syms::*,
        ModifiedKeySym,
    },
    quit, reload, set_show_titles, switch_to_vt, Axis, Direction,
};

use crate::{actions, bar, exec};

/// Binds a shortcut that fires again while the key is held. The toml side
/// needs a `complex-shortcut` with `repeat = true` for this.
fn bind_repeating<F: FnMut() + 'static>(seat: Seat, sym: impl Into<ModifiedKeySym>, f: F) {
    let sym = sym.into();
    seat.bind(sym, f);
    seat.set_repeat_bind(sym, true);
}

// --- keys ---

/// Indices into a `resize` delta, in the order the compositor takes them.
const DX1: usize = 0;
const DY1: usize = 1;
const DX2: usize = 2;
const DY2: usize = 3;

/// `field`/`sign` describe which edge of a window the direction resizes and
/// in which direction that edge grows, see the resize mode below.
struct Dir {
    key: KeySym,
    arrow: KeySym,
    dir: Direction,
    field: usize,
    sign: i32,
}

impl Dir {
    /// The delta that moves this direction's edge outwards by `amount`, or
    /// inwards for a negative one.
    fn resize(&self, amount: i32) -> [i32; 4] {
        let mut delta = [0; 4];
        delta[self.field] = self.sign * amount;
        delta
    }
}

const DIRS: [Dir; 4] = [
    Dir {
        key: SYM_h,
        arrow: SYM_Left,
        dir: Direction::Left,
        field: DX1,
        sign: -1,
    },
    Dir {
        key: SYM_j,
        arrow: SYM_Down,
        dir: Direction::Down,
        field: DY2,
        sign: 1,
    },
    Dir {
        key: SYM_k,
        arrow: SYM_Up,
        dir: Direction::Up,
        field: DY1,
        sign: -1,
    },
    Dir {
        key: SYM_l,
        arrow: SYM_Right,
        dir: Direction::Right,
        field: DX2,
        sign: 1,
    },
];

const WORKSPACES: [(KeySym, &str); 10] = [
    (SYM_0, "0"),
    (SYM_1, "1"),
    (SYM_2, "2"),
    (SYM_3, "3"),
    (SYM_4, "4"),
    (SYM_5, "5"),
    (SYM_6, "6"),
    (SYM_7, "7"),
    (SYM_8, "8"),
    (SYM_9, "9"),
];

const VTS: [KeySym; 12] = [
    SYM_F1, SYM_F2, SYM_F3, SYM_F4, SYM_F5, SYM_F6, SYM_F7, SYM_F8, SYM_F9, SYM_F10, SYM_F11,
    SYM_F12,
];

const RESIZE_AMOUNT: i32 = 10;

// --- modes ---

/// A set of shortcuts that is layered on top of the normal ones until the
/// mode is left again.
///
/// Jay's config API has no mode stack, so entering a mode binds its shortcuts
/// and rebinds its own key to leave it again. The normal bindings are never
/// torn down, so there is no state to rebuild; only one mode at a time can be
/// active, which is all that is ever entered. The toml side pushes onto the
/// compositor's mode stack instead, but the two behave the same as long as
/// modes are not nested.
struct Mode {
    name: &'static str,
    key: ModifiedKeySym,
    shortcuts: Vec<(ModifiedKeySym, Shortcut)>,
}

enum Shortcut {
    /// Leaves the mode, then runs.
    Leave(Rc<dyn Fn()>),
    /// Keeps the mode active and fires again while the key is held.
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

    fn leave(mut self, sym: impl Into<ModifiedKeySym>, f: impl Fn() + 'static) -> Self {
        self.shortcuts
            .push((sym.into(), Shortcut::Leave(Rc::new(f))));
        self
    }

    fn repeat(mut self, sym: impl Into<ModifiedKeySym>, f: impl Fn() + 'static) -> Self {
        self.shortcuts
            .push((sym.into(), Shortcut::Repeat(Rc::new(f))));
        self
    }

    /// Binds the key that enters the mode. This is also how a mode that has
    /// just been left returns to being enterable.
    fn install(self: &Rc<Self>, seat: Seat) {
        let mode = self.clone();
        seat.bind(self.key, move || mode.enter(seat));
    }

    fn enter(self: &Rc<Self>, seat: Seat) {
        for (sym, shortcut) in &self.shortcuts {
            match shortcut {
                Shortcut::Leave(f) => {
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
        self.install(seat);

        bar::set_mode(None);
    }
}

fn mirror_mode(seat: Seat) {
    let present = |action: &'static str| move || exec("wl-present", &[action]);

    Rc::new(
        Mode::new("mirror", LOGO | SYM_m)
            .leave(SYM_m, present("mirror"))
            .leave(SYM_c, present("custom"))
            .leave(SYM_f, present("toggle-freeze"))
            .leave(SYM_z, present("freeze"))
            .leave(SHIFT | SYM_z, present("unfreeze"))
            .leave(SYM_o, present("set-output"))
            .leave(SYM_r, present("set-region"))
            .leave(SHIFT | SYM_r, present("unset-region"))
            .leave(SYM_s, present("set-scaling")),
    )
    .install(seat);
}

fn resize_mode(seat: Seat) {
    let resize_by = |delta: [i32; 4]| move || seat.resize(delta[0], delta[1], delta[2], delta[3]);

    let mode = DIRS
        .iter()
        .fold(Mode::new("resize", LOGO | SYM_r), |mode, dir| {
            let (grow, shrink) = (dir.resize(RESIZE_AMOUNT), dir.resize(-RESIZE_AMOUNT));
            mode.repeat(dir.key, resize_by(grow))
                .repeat(dir.arrow, resize_by(grow))
                .repeat(SHIFT | dir.key, resize_by(shrink))
                .repeat(SHIFT | dir.arrow, resize_by(shrink))
        });

    Rc::new(mode).install(seat);
}

fn system_mode(seat: Seat) {
    Rc::new(
        Mode::new("system", LOGO | SYM_p)
            .leave(SYM_l, actions::lock)
            .leave(SYM_s, || exec("systemctl", &["poweroff"]))
            .leave(SYM_r, || exec("systemctl", &["reboot"]))
            .leave(SYM_h, actions::suspend)
            .leave(SYM_i, actions::toggle_idle_inhibitor),
    )
    .install(seat);
}

// --- screenshots ---

/// Hands a region to the screenshot script, which is what the toml side ends
/// up running as well. Unlike that side, which has to derive its geometry
/// from the focused window, this can also capture an empty workspace or
/// output; an empty region means there was nothing to capture at all.
fn screenshot((x, y): (i32, i32), (width, height): (i32, i32)) {
    if width > 0 && height > 0 {
        exec(
            "jay-screenshot",
            &["region", &format!("{x},{y} {width}x{height}")],
        );
    }
}

/// Moves the current workspace to the neighbouring output, if there is one.
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

    // --- directions ---
    //
    // Everything that is worth holding down rather than tapping - navigating
    // focus, dragging a window or workspace along, stepping the volume - is
    // bound with `bind_repeating`.
    for dir in DIRS {
        let direction = dir.dir;
        for sym in [dir.key, dir.arrow] {
            bind_repeating(seat, LOGO | sym, move || seat.focus(direction));
            bind_repeating(seat, LOGO | SHIFT | sym, move || seat.move_(direction));
            bind_repeating(seat, LOGO | SHIFT | CTRL | sym, move || {
                move_to_output(seat, direction)
            });
        }
    }

    // --- workspaces ---
    for (sym, name) in WORKSPACES {
        seat.bind(LOGO | sym, move || seat.show_workspace(get_workspace(name)));
        seat.bind(LOGO | SHIFT | sym, move || {
            seat.set_workspace(get_workspace(name))
        });
    }

    // --- switch to VT ---
    for (i, sym) in VTS.into_iter().enumerate() {
        let n = i as u32 + 1;
        seat.bind(CTRL | ALT | sym, move || switch_to_vt(n));
    }

    // --- compositor ---
    seat.bind(LOGO | SHIFT | SYM_q, quit);
    // Reloading re-runs this configuration, which also seeds the bar again.
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

    // Marks; the next key press identifies the mark. The toml side binds
    // `tile-major`/`split-major` to these keys instead, neither of which this
    // version of the jay-config crate knows about.
    seat.bind(LOGO | SYM_y, move || seat.jump_to_mark(None));
    seat.bind(LOGO | SHIFT | SYM_y, move || seat.create_mark(None));

    // --- focus ---
    bind_repeating(seat, LOGO | SYM_Tab, move || {
        seat.focus_history(Timeline::Newer)
    });
    bind_repeating(seat, LOGO | SHIFT | SYM_Tab, move || {
        seat.focus_history(Timeline::Older)
    });
    // layer above
    bind_repeating(seat, LOGO | SYM_Prior, move || {
        seat.focus_layer_rel(LayerDirection::Above)
    });
    // layer below
    bind_repeating(seat, LOGO | SYM_Next, move || {
        seat.focus_layer_rel(LayerDirection::Below)
    });
    bind_repeating(seat, LOGO | SYM_g, move || seat.focus_parent());
    seat.bind(LOGO | SYM_Delete, move || seat.focus_tiles());
    seat.bind(LOGO | SYM_c, move || seat.warp_mouse_to_focus());

    // --- modes ---
    mirror_mode(seat);
    resize_mode(seat);
    system_mode(seat);

    // --- audio (wireplumber) ---
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

    // --- player ---
    seat.bind(SYM_XF86AudioPlay, || exec("playerctl", &["play-pause"]));
    seat.bind(SYM_XF86AudioPause, || exec("playerctl", &["play-pause"]));
    seat.bind(SYM_XF86AudioNext, || exec("playerctl", &["next"]));
    seat.bind(SYM_XF86AudioPrev, || exec("playerctl", &["previous"]));
    seat.bind(SYM_XF86AudioStop, || exec("playerctl", &["stop"]));

    // --- screenshot ---
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

    // --- launch ---
    seat.bind(LOGO | SYM_Return, || exec("runapp", &["footclient"]));
    seat.bind(LOGO | SHIFT | SYM_Return, || exec("runapp", &["foot"]));
    seat.bind(LOGO | SYM_d, || exec("fuzzel", &[]));
    seat.bind(LOGO | SYM_a, || exec("swaync-client", &["-t"]));
    seat.bind(LOGO | SHIFT | SYM_v, || exec("jay-clipboard-history", &[]));
}
