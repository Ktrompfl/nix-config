//! What a calculator has to say, in the panel's language rather than its own.
//!
//! A backend translates into these types and the frontend draws them. Anything
//! a backend leaves out is `None` or empty and simply is not drawn, which is
//! how it says "this does not apply to me" -- so a calculator with no blind
//! mode needs no special case anywhere.

/// How far along the calculator is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Status {
    /// Nothing has been measured yet.
    #[default]
    Waiting,
    /// There is an answer, in `predictions`.
    Solved,
    /// The measurements do not agree on anywhere.
    Failed,
}

/// A stronghold the calculator considers possible.
#[derive(Clone, Debug)]
pub struct Prediction {
    /// Between nought and one.
    pub certainty: f64,
    pub chunk: (i32, i32),
    /// In overworld blocks, whatever dimension the player is in.
    pub overworld_distance: f64,
}

/// One throw, as the calculator has it.
#[derive(Clone, Debug)]
pub struct ThrowReport {
    pub x: f64,
    pub z: f64,
    /// The angle actually used, after adjustment.
    pub angle: f64,
    /// How far it was adjusted, in degrees and in steps.
    pub correction: f64,
    pub correction_steps: i32,
    /// How far this throw is from pointing at the best guess.
    pub error: f64,
}

/// Where the player last said they were.
#[derive(Clone, Debug, Default)]
pub struct Player {
    pub position: Option<(f64, f64)>,
    pub horizontal_angle: Option<f64>,
    pub in_nether: bool,
}

/// How good a blind position is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Quality {
    Excellent,
    Good,
    Okay,
    Poor,
    Bad,
    OutOfRange,
}

impl Quality {
    pub fn label(self) -> &'static str {
        match self {
            Quality::Excellent => "Excellent",
            Quality::Good => "Good",
            Quality::Okay => "Okay",
            Quality::Poor => "Bad, in ring",
            Quality::Bad => "Bad",
            Quality::OutOfRange => "Not in ring",
        }
    }

    /// Where this sits on the nought-to-one scale the panel colours by, so
    /// that a quality and a certainty are shaded alike.
    pub fn goodness(self) -> f64 {
        match self {
            Quality::Excellent => 1.0,
            Quality::Good => 0.9,
            Quality::Okay => 0.7,
            Quality::Poor => 0.5,
            Quality::Bad => 0.2,
            Quality::OutOfRange => 0.0,
        }
    }
}

/// What to do before the first throw.
#[derive(Clone, Debug)]
pub struct Blind {
    pub nether: (f64, f64),
    pub quality: Quality,
    /// Between nought and one, of landing inside `highroll_threshold`.
    pub highroll_probability: f64,
    pub highroll_threshold: f64,
    /// Somewhere better to stand: how far, and which way. Absent when there
    /// is nowhere worth walking to.
    pub improve: Option<Improvement>,
}

#[derive(Clone, Copy, Debug)]
pub struct Improvement {
    pub distance: f64,
    pub direction: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Severity {
    Info,
    #[default]
    Warning,
    Error,
}

/// Something the calculator wants to say, as text. Whatever markup it used to
/// say it is the backend's business and has been dealt with by the time this
/// exists.
#[derive(Clone, Debug)]
pub struct Message {
    pub severity: Severity,
    pub text: String,
}

/// Everything the calculator currently has to show.
#[derive(Clone, Debug, Default)]
pub struct Solution {
    pub status: Status,
    pub predictions: Vec<Prediction>,
    pub throws: Vec<ThrowReport>,
    pub player: Player,
    pub blind: Option<Blind>,
    pub messages: Vec<Message>,
}

impl Solution {
    /// The line the panel shows in place of a table, if any.
    pub fn placeholder(&self) -> Option<&str> {
        match self.status {
            Status::Failed => Some("Could not determine the stronghold chunk."),
            Status::Waiting if self.blind.is_none() => Some("Waiting for F3+C..."),
            _ if self.predictions.is_empty() && self.blind.is_none() => {
                Some("Waiting for F3+C...")
            }
            _ => None,
        }
    }

    /// Whether there is nothing worth putting on screen at all.
    pub fn is_idle(&self) -> bool {
        self.status == Status::Waiting
            && self.predictions.is_empty()
            && self.throws.is_empty()
            && self.blind.is_none()
            && self.messages.is_empty()
    }
}

impl Prediction {
    /// The block the stronghold is expected in, which is the chunk's centre.
    pub fn block(&self) -> (i32, i32) {
        (self.chunk.0 * 16 + 8, self.chunk.1 * 16 + 8)
    }

    /// The nether coordinates above it: overworld blocks over eight, which
    /// for a whole chunk is just the chunk doubled.
    pub fn nether(&self) -> (i32, i32) {
        (self.chunk.0 * 2, self.chunk.1 * 2)
    }

    /// Distance in whichever dimension the player is standing in.
    pub fn distance(&self, player: &Player) -> f64 {
        if player.in_nether {
            self.overworld_distance / 8.0
        } else {
            self.overworld_distance
        }
    }

    /// The yaw to face to walk towards it, in Minecraft's convention: zero
    /// looks along +z, and yaw grows towards -x.
    pub fn travel_angle(&self, player: &Player) -> Option<f64> {
        let (px, pz) = player.position?;
        let (bx, bz) = self.block();
        Some(normalize_angle(
            -(bx as f64 - px).atan2(bz as f64 - pz).to_degrees(),
        ))
    }

    /// How far the player has to turn to face it.
    pub fn travel_angle_delta(&self, player: &Player) -> Option<f64> {
        Some(normalize_angle(
            self.travel_angle(player)? - player.horizontal_angle?,
        ))
    }
}

/// Folds an angle into (-180, 180].
pub fn normalize_angle(mut degrees: f64) -> f64 {
    while degrees <= -180.0 {
        degrees += 360.0;
    }
    while degrees > 180.0 {
        degrees -= 360.0;
    }
    degrees
}
