#![forbid(unsafe_code)]

pub use geom::{Color, Rect, Region};
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WindowId(pub u64);

#[derive(Clone, Debug, Default)]
pub struct WindowInfo {
    pub app_id: String,
    pub title: String,
    pub tag: String,
    pub parent: Option<WindowId>,
}

pub type OutputId = u32;

#[derive(Clone, Debug, PartialEq)]
pub struct Output {
    pub id: OutputId,
    pub full: Rect,
    pub usable: Rect,
    pub workspaces: Vec<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Placed {
    pub window: WindowId,
    pub rect: Rect,
    pub activated: bool,
    pub fullscreen: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Deco {
    Fill(Rect, Color),
    Text { rect: Rect, text: Rc<str>, font: Rc<str>, fg: Color, bg: Color },
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Arrangement {
    pub background: Color,
    pub decorations: Vec<Deco>,
    pub windows: Vec<Placed>,
    pub focus: Option<WindowId>,
    pub pointer: Option<Rect>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Hit {
    Window(WindowId),
    Output(OutputId),
    #[default]
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cause {
    Pointer,
    Keyboard,
    Client,
    Activation,
    User,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WorkspaceInfo {
    pub name: String,
    pub output: Option<OutputId>,
    pub active: bool,
    pub urgent: bool,
}

pub trait WindowManager {
    fn outputs_changed(&mut self, outputs: &[Output]);
    fn window_added(&mut self, w: WindowId, info: &WindowInfo);
    fn window_removed(&mut self, w: WindowId);
    fn window_changed(&mut self, _w: WindowId, _info: &WindowInfo) -> bool {
        true
    }
    fn focus(&mut self, w: WindowId, cause: Cause);
    fn fullscreen(&mut self, _w: WindowId, _on: bool, _output: Option<OutputId>) {}
    fn pointer(&mut self, _at: Hit, _cx: &mut Cx) -> bool {
        false
    }
    fn button(&mut self, _at: Hit, _button: u32, _pressed: bool, _cx: &mut Cx) -> bool {
        false
    }
    fn workspaces(&self) -> Vec<WorkspaceInfo> {
        vec![]
    }
    fn activate_workspace(&mut self, _name: &str) {}
    fn arrange(&self, out: &mut Arrangement);
}

#[derive(Debug)]
pub enum Request {
    Spawn(Vec<String>),
    Close(WindowId),
    Quit,
    SwitchVt(i32),
    StepVt(i32),
    FreezeMirrors(bool),
}

#[derive(Default)]
pub struct Cx {
    pub requests: Vec<Request>,
    mode: Option<Option<&'static str>>,
}

impl Cx {
    pub fn spawn(&mut self, command: &[&str]) {
        self.requests.push(Request::Spawn(command.iter().map(|s| s.to_string()).collect()));
    }

    pub fn close(&mut self, w: Option<WindowId>) {
        self.requests.extend(w.map(Request::Close));
    }

    pub fn quit(&mut self) {
        self.requests.push(Request::Quit);
    }

    pub fn freeze_mirrors(&mut self, freeze: bool) {
        self.requests.push(Request::FreezeMirrors(freeze));
    }

    pub fn switch_vt(&mut self, vt: i32) {
        self.requests.push(Request::SwitchVt(vt));
    }

    pub fn step_vt(&mut self, delta: i32) {
        self.requests.push(Request::StepVt(delta));
    }

    pub fn enter_mode(&mut self, mode: &'static str) {
        self.mode = Some(Some(mode));
    }

    pub fn exit_mode(&mut self) {
        self.mode = Some(None);
    }
}

pub type Mods = u32;
pub const SHIFT: Mods = 1;
pub const CTRL: Mods = 4;
pub const ALT: Mods = 8;
pub const LOGO: Mods = 64;

pub mod key {
    pub const DIGITS: [u32; 10] = [0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39];
    pub const B: u32 = 0x62;
    pub const D: u32 = 0x64;
    pub const E: u32 = 0x65;
    pub const F: u32 = 0x66;
    pub const H: u32 = 0x68;
    pub const J: u32 = 0x6a;
    pub const K: u32 = 0x6b;
    pub const L: u32 = 0x6c;
    pub const G: u32 = 0x67;
    pub const N: u32 = 0x6e;
    pub const P: u32 = 0x70;
    pub const R: u32 = 0x72;
    pub const TAB: u32 = 0xff09;
    pub const LEFT: u32 = 0xff51;
    pub const UP: u32 = 0xff52;
    pub const RIGHT: u32 = 0xff53;
    pub const DOWN: u32 = 0xff54;
    pub const Q: u32 = 0x71;
    pub const V: u32 = 0x76;
    pub const Z: u32 = 0x7a;
    pub const ENTER: u32 = 0xff0d;
    pub const ESC: u32 = 0xff1b;
    pub const PAGE_UP: u32 = 0xff55;
    pub const PAGE_DOWN: u32 = 0xff56;
    pub const DELETE: u32 = 0xffff;
    pub const FN: [u32; 12] =
        [0xffbe, 0xffbf, 0xffc0, 0xffc1, 0xffc2, 0xffc3, 0xffc4, 0xffc5, 0xffc6, 0xffc7, 0xffc8, 0xffc9];
}

type Action<W> = Box<dyn Fn(&mut W, &mut Cx)>;

struct Binding<W> {
    mods: Mods,
    keysym: u32,
    repeat: bool,
    locked: bool,
    action: Action<W>,
}

pub struct Bindings<W> {
    keys: Vec<Binding<W>>,
    modes: Vec<(&'static str, Bindings<W>)>,
}

impl<W> Default for Bindings<W> {
    fn default() -> Self {
        Self { keys: vec![], modes: vec![] }
    }
}

impl<W> Bindings<W> {
    pub fn bind(&mut self, mods: Mods, keysym: u32, action: impl Fn(&mut W, &mut Cx) + 'static) {
        self.keys.push(Binding { mods, keysym, repeat: false, locked: false, action: Box::new(action) });
    }

    pub fn repeat(&mut self, mods: Mods, keysym: u32, action: impl Fn(&mut W, &mut Cx) + 'static) {
        self.keys.push(Binding { mods, keysym, repeat: true, locked: false, action: Box::new(action) });
    }

    pub fn always(&mut self, mods: Mods, keysym: u32, action: impl Fn(&mut W, &mut Cx) + 'static) {
        self.keys.push(Binding { mods, keysym, repeat: false, locked: true, action: Box::new(action) });
    }

    pub fn mode(&mut self, name: &'static str) -> &mut Bindings<W> {
        let index = match self.modes.iter().position(|m| m.0 == name) {
            Some(index) => index,
            None => {
                self.modes.push((name, Bindings::default()));
                self.modes.len() - 1
            }
        };
        &mut self.modes[index].1
    }
}

pub trait Policy {
    fn wm(&mut self) -> &mut dyn WindowManager;
    fn key(&mut self, mods: Mods, keysym: u32, locked: bool, cx: &mut Cx) -> Option<bool>;
}

pub struct Shell<W> {
    pub wm: W,
    pub bindings: Bindings<W>,
    mode: Option<&'static str>,
}

impl<W> Shell<W> {
    pub fn new(wm: W, bindings: Bindings<W>) -> Self {
        Self { wm, bindings, mode: None }
    }
}

impl<W: WindowManager> Policy for Shell<W> {
    fn wm(&mut self) -> &mut dyn WindowManager {
        &mut self.wm
    }

    fn key(&mut self, mods: Mods, keysym: u32, locked: bool, cx: &mut Cx) -> Option<bool> {
        let mods = mods & (SHIFT | CTRL | ALT | LOGO);
        let mode = self.mode.and_then(|name| self.bindings.modes.iter().find(|m| m.0 == name)).map(|m| &m.1);
        let binding = match mode {
            Some(mode) => find(mode, mods, keysym, locked).or_else(|| find(&self.bindings, mods, keysym, true)),
            None => find(&self.bindings, mods, keysym, locked),
        }?;
        (binding.action)(&mut self.wm, cx);
        if let Some(mode) = cx.mode.take() {
            self.mode = mode;
        }
        Some(binding.repeat)
    }
}

fn find<W>(b: &Bindings<W>, mods: Mods, keysym: u32, locked: bool) -> Option<&Binding<W>> {
    b.keys.iter().find(|b| (b.mods, b.keysym) == (mods, keysym) && (b.locked || !locked))
}

pub fn split(r: Rect, weights: &[f64], i: usize, horizontal: bool) -> Rect {
    let total: f64 = weights.iter().sum();
    let edge = |len: i32, upto: usize| (f64::from(len) * weights[..upto].iter().sum::<f64>() / total).round() as i32;
    match horizontal {
        true => Rect::new(r.x + edge(r.w, i), r.y, edge(r.w, i + 1) - edge(r.w, i), r.h),
        false => Rect::new(r.x, r.y + edge(r.h, i), r.w, edge(r.h, i + 1) - edge(r.h, i)),
    }
}

pub fn neighbor<T>(from: Rect, dir: Dir, candidates: impl IntoIterator<Item = (T, Rect)>) -> Option<T> {
    let (fx, fy) = from.center();
    let distance = |r: Rect| {
        let (x, y) = r.center();
        let (along, across) = match dir {
            Dir::Left => (fx - x, y - fy),
            Dir::Right => (x - fx, y - fy),
            Dir::Up => (fy - y, x - fx),
            Dir::Down => (y - fy, x - fx),
        };
        (along > 0).then_some(along + 2 * across.abs())
    };
    candidates.into_iter().filter_map(|(w, r)| Some((distance(r)?, w))).min_by_key(|c| c.0).map(|(_, w)| w)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Nothing;

    impl WindowManager for Nothing {
        fn outputs_changed(&mut self, _: &[Output]) {}
        fn window_added(&mut self, _: WindowId, _: &WindowInfo) {}
        fn window_removed(&mut self, _: WindowId) {}
        fn focus(&mut self, _: WindowId, _: Cause) {}
        fn arrange(&self, _: &mut Arrangement) {}
    }

    #[test]
    fn only_always_bindings_fire_while_locked() {
        let mut b = Bindings::default();
        b.bind(0, key::DELETE, |_, cx| cx.quit());
        b.always(0, key::PAGE_UP, |_, cx| cx.step_vt(-1));
        let mut shell = Shell::new(Nothing, b);
        let mut cx = Cx::default();
        assert_eq!(shell.key(0, key::DELETE, true, &mut cx), None);
        assert_eq!(shell.key(0, key::PAGE_UP, true, &mut cx), Some(false));
        assert_eq!(shell.key(0, key::DELETE, false, &mut cx), Some(false));
        assert!(matches!(cx.requests[..], [Request::StepVt(-1), Request::Quit]));
    }

    #[test]
    fn always_bindings_fire_in_modes() {
        let mut b = Bindings::default();
        b.always(CTRL, key::FN[0], |_, cx| cx.switch_vt(1));
        b.bind(0, key::DELETE, |_, cx| cx.quit());
        b.bind(0, key::R, |_, cx| cx.enter_mode("resize"));
        b.mode("resize").bind(0, key::ESC, |_, cx| cx.exit_mode());
        let mut shell = Shell::new(Nothing, b);
        let mut cx = Cx::default();
        shell.key(0, key::R, false, &mut cx);
        assert_eq!(shell.key(0, key::DELETE, false, &mut cx), None);
        assert_eq!(shell.key(CTRL, key::FN[0], true, &mut cx), Some(false));
        assert_eq!(shell.key(0, key::ESC, true, &mut cx), None);
        assert!(matches!(cx.requests[..], [Request::SwitchVt(1)]));
    }
}
