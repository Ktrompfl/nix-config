#![forbid(unsafe_code)]

use std::os::fd::OwnedFd;

pub type OutputId = u32;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Mode {
    pub width: i32,
    pub height: i32,
    pub refresh: u32,
}

#[derive(Clone, Debug, Default)]
pub struct OutputInfo {
    pub id: OutputId,
    pub name: String,
    pub make: String,
    pub model: String,
    pub serial: String,
    pub modes: Vec<Mode>,
    pub preferred: Option<Mode>,
    pub mode: Mode,
    pub scale: f64,
}

impl OutputInfo {
    pub fn identity(&self) -> [&str; 4] {
        [&self.name, &self.make, &self.model, &self.serial]
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScrollAxis {
    pub value: f64,
    pub value120: i32,
    pub inverted: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Scroll {
    pub source: u32,
    pub axes: [Option<ScrollAxis>; 2],
}

impl Scroll {
    pub const WHEEL: u32 = 0;
    pub const FINGER: u32 = 1;
    pub const CONTINUOUS: u32 = 2;
    pub const WHEEL_TILT: u32 = 3;
}

#[derive(Clone, Copy, Debug)]
pub struct Flip {
    pub time: std::time::Duration,
    pub sequence: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GestureKind {
    Swipe,
    Pinch,
    Hold,
}

#[derive(Clone, Copy, Debug)]
pub enum Gesture {
    Begin(GestureKind, u32),
    Update { dx: f64, dy: f64, scale: f64, rotation: f64 },
    End(bool),
}

#[derive(Debug)]
pub enum Event {
    Output(OutputEvent),
    Input(InputEvent),
    Frame { output: OutputId, flip: Option<Flip> },
    Session(bool),
    Close,
}

#[derive(Debug)]
pub enum OutputEvent {
    Added(OutputInfo),
    Changed(OutputInfo),
    Removed(OutputId),
    GammaFailed(OutputId),
}

#[derive(Debug)]
pub enum InputEvent {
    Keymap(OwnedFd, u32),
    Key { key: u32, pressed: bool },
    Modifiers([u32; 4]),
    Motion { output: OutputId, x: f64, y: f64 },
    Button { button: u32, pressed: bool },
    Scroll(Scroll),
    PointerLeft,
    RelativeMotion { delta: [f64; 4], usec: u64 },
    Gesture(Gesture),
}
