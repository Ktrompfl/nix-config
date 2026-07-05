use jay_config::{
    exec::Command,
    input::Seat,
    keyboard::{
        mods::{LOGO, SHIFT},
        syms::*,
    },
};

use crate::{generated::JAY_MODE_SCRIPT, shortcuts};

fn notify_push(name: &str) {
    Command::new(JAY_MODE_SCRIPT).arg(name).spawn();
}

fn notify_pop() {
    Command::new(JAY_MODE_SCRIPT).spawn();
}

// --- mirror mode ---

pub fn push_mirror(seat: Seat) {
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
    shortcuts::setup();
    notify_pop();
}

// --- resize mode ---

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

pub fn push_resize(seat: Seat) {
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
    shortcuts::setup();
    notify_pop();
}

// --- system mode ---

pub fn push_system(seat: Seat) {
    seat.bind(LOGO | SYM_p, move || pop_system(seat));
    seat.bind(SYM_Escape, move || pop_system(seat));
    seat.bind(SYM_l, move || {
        pop_system(seat);
        Command::new("swaylock").privileged().spawn();
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
    shortcuts::setup();
    notify_pop();
}
