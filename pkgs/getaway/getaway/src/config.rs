use server::{Accel, Caps, Config, DeviceKind, InputConfig, InputDevice, KeymapNames, OutputConfig, OutputInfo, caps};
use wm::{
    ALT, Bindings, CTRL, Dir, LOGO, SHIFT,
    key::{self, DIGITS},
};
use wm_tree::{Theme, TreeWm};

pub type Wm = TreeWm;

const RESIZE_STEP: i32 = 10;

pub fn server() -> Config {
    Config {
        client_caps,
        xwayland: std::env::var_os("GETAWAY_XWAYLAND").is_none_or(|v| v != "0"),
        keymap: Some(KeymapNames { layout: "us", variant: "altgr-intl", options: &["caps:escape"] }),
        input,
        outputs,
        nested_outputs: std::env::var("GETAWAY_NESTED_OUTPUTS").ok().and_then(|n| n.parse().ok()).unwrap_or(1),
    }
}

fn nested() -> bool {
    std::env::var_os("WAYLAND_DISPLAY").is_some()
}

fn modifier() -> u32 {
    if nested() { ALT } else { LOGO }
}

fn terminal() -> &'static [&'static str] {
    if nested() { &["foot"] } else { &["runapp", "--scope", "--", "foot"] }
}

fn launcher() -> &'static [&'static str] {
    if nested() { &["fuzzel", "--launch-prefix=env"] } else { &["fuzzel", "--launch-prefix=runapp --scope --"] }
}

fn input(device: &InputDevice) -> InputConfig {
    let flat = Some(Accel::Flat(0.0));
    match device.kind {
        DeviceKind::Touchpad => {
            InputConfig { tap: Some(true), natural_scroll: Some(false), accel: flat, ..Default::default() }
        }
        DeviceKind::Pointer => InputConfig { accel: flat, ..Default::default() },
        _ => InputConfig::default(),
    }
}

const LAPTOP_PANEL: &str = "eDP-1";

fn outputs(info: &OutputInfo, all: &[OutputInfo]) -> OutputConfig {
    let laptop = all.iter().any(|o| o.name == LAPTOP_PANEL);
    let (workspaces, mirror, vrr) = match (info.name.as_str(), info.model.as_str()) {
        (LAPTOP_PANEL, _) => (vec![1, 2, 3, 4, 5], None, false),
        _ if laptop => (vec![], Some(LAPTOP_PANEL.into()), false),
        (_, "VG270U P") => (vec![1, 2, 3, 4, 5], None, true),
        (_, "BenQ GL2480") => (vec![6, 7, 8, 9, 0], None, false),
        _ => (vec![], None, false),
    };
    OutputConfig { enabled: true, workspaces, mirror, vrr, ..Default::default() }
}

fn theme_file() -> Option<(std::path::PathBuf, toml::Table)> {
    let home = || std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config"));
    let path =
        std::env::var_os("XDG_CONFIG_HOME").map(std::path::PathBuf::from).or_else(home)?.join("getaway/theme.toml");
    let text = std::fs::read_to_string(&path).inspect_err(|e| log::info!("no theme {}: {e}", path.display())).ok()?;
    let table = text.parse().inspect_err(|e| log::warn!("invalid theme {}: {e}", path.display())).ok()?;
    Some((path, table))
}

pub fn theme() -> Theme {
    let file = theme_file();
    let value = |key: &str| file.as_ref().and_then(|(_, t)| t.get(key)?.as_str().map(str::to_owned));
    let color = |key: &str, default: u32| {
        let parsed = value(key).map(|v| (u32::from_str_radix(v.trim_start_matches('#'), 16), v));
        match parsed {
            Some((Ok(rgb), v)) if v.trim_start_matches('#').len() == 6 => 0xff00_0000 | rgb,
            Some((_, v)) => {
                log::warn!("invalid color {key} = {v:?}");
                default
            }
            None => default,
        }
    };
    Theme {
        background: color("base00", 0xff1d2021),
        border: color("base03", 0xff3c3836),
        border_focused: color("base0d", 0xff83a598),
        title: color("base01", 0xff32302f),
        title_focused: color("base0d", 0xff83a598),
        title_text: color("base04", 0xffa89984),
        title_text_focused: color("base00", 0xff1d2021),
        font: value("sans_font").unwrap_or_else(|| "sans-serif 9".into()).into(),
        border_width: 1,
        title_height: 18,
        show_titles: true,
    }
}

const DIRS: [(u32, u32, Dir); 4] = [
    (key::H, key::LEFT, Dir::Left),
    (key::J, key::DOWN, Dir::Down),
    (key::K, key::UP, Dir::Up),
    (key::L, key::RIGHT, Dir::Right),
];

pub fn bindings(b: &mut Bindings<Wm>) {
    let m = modifier();
    for (letter, arrow, dir) in DIRS {
        for k in [letter, arrow] {
            b.repeat(m, k, move |wm, _| wm.focus_dir(dir));
            b.repeat(m | SHIFT, k, move |wm, _| wm.move_dir(dir));
            b.repeat(m | SHIFT | CTRL, k, move |wm, _| wm.move_to_output(dir));
        }
    }
    for (workspace, k) in DIGITS.into_iter().enumerate() {
        b.bind(m, k, move |wm, _| wm.show(workspace));
        b.bind(m | SHIFT, k, move |wm, _| wm.move_to(workspace));
    }
    b.bind(m, key::ENTER, |_, cx| cx.spawn(terminal()));
    b.bind(m, key::D, |_, cx| cx.spawn(launcher()));
    b.bind(m, key::P, |_, cx| cx.spawn(&["power-menu"]));
    b.bind(m, key::Q, |wm, cx| cx.close(wm.focused()));
    b.bind(m, key::F, |wm, _| wm.toggle_fullscreen());
    b.bind(m, key::V, |wm, _| wm.toggle_split());
    b.bind(m, key::N, |wm, _| wm.toggle_tabbed());
    b.bind(m, key::B, |wm, _| wm.split());
    b.repeat(m, key::G, |wm, _| wm.focus_parent());
    b.repeat(m, key::TAB, |wm, _| wm.focus_previous());
    b.bind(m | SHIFT, key::E, |_, cx| cx.quit());
    b.bind(m, key::Z, |_, cx| cx.freeze_mirrors(true));
    b.bind(m | SHIFT, key::Z, |_, cx| cx.freeze_mirrors(false));
    for (vt, k) in (1..).zip(key::FN) {
        b.always(CTRL | ALT, k, move |_, cx| cx.switch_vt(vt));
    }
    escape(b);
    resize_mode(b);
}

fn escape(b: &mut Bindings<Wm>) {
    if nested() {
        b.bind(0, key::DELETE, |_, cx| cx.quit());
        b.always(0, key::PAGE_UP, |_, cx| cx.step_vt(-1));
        b.always(0, key::PAGE_DOWN, |_, cx| cx.step_vt(1));
    }
}

fn resize_mode(b: &mut Bindings<Wm>) {
    let m = modifier();
    b.bind(m, key::R, |_, cx| cx.enter_mode("resize"));
    let mode = b.mode("resize");
    for (letter, arrow, dir) in DIRS {
        for k in [letter, arrow] {
            mode.repeat(0, k, move |wm, _| wm.resize(dir, RESIZE_STEP));
            mode.repeat(SHIFT, k, move |wm, _| wm.resize(dir, -RESIZE_STEP));
        }
    }
    for k in [key::ESC, key::R] {
        mode.bind(0, k, |_, cx| cx.exit_mode());
    }
    mode.bind(m, key::R, |_, cx| cx.exit_mode());
    escape(mode);
}

pub fn client_caps(executable: &str) -> Caps {
    match executable {
        "swaylock" => caps::LAYER_SHELL | caps::SESSION_LOCK,
        "fuzzel" | "swaync" | "awww-daemon" => caps::LAYER_SHELL,
        "wl-copy" | "wl-paste" | "wl-clip-persist" => caps::DATA_CONTROL,
        "ninjabrain-box" => caps::LAYER_SHELL | caps::DATA_CONTROL | caps::FOREIGN_TOPLEVEL,
        "grim" | "xdg-desktop-portal-wlr" => caps::SCREENCOPY | caps::FOREIGN_TOPLEVEL,
        "wl-mirror" | "wl-present" => caps::SCREENCOPY,
        "swayidle" => caps::IDLE_NOTIFY,
        "wlopm" => caps::OUTPUT_POWER,
        "waybar" => caps::LAYER_SHELL | caps::FOREIGN_TOPLEVEL | caps::WORKSPACES,
        "i3bar-wl" => caps::LAYER_SHELL | caps::WORKSPACES,
        "wlrctl" => caps::LAYER_SHELL | caps::FOREIGN_TOPLEVEL | caps::WORKSPACES | caps::VIRTUAL_INPUT,
        "wtype" => caps::VIRTUAL_INPUT,
        "wlr-randr" | "kanshi" | "shikane" | "wdisplays" => caps::OUTPUT_MANAGEMENT,
        "wlsunset" | "gammastep" => caps::GAMMA,
        "wayland-pipewire-idle-inhibit" => caps::LAYER_SHELL,
        _ => caps::NONE,
    }
}
