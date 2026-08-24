//! The box's own language, which both halves speak and neither owns.
//!
//! [`Measurement`] is what the game produces, [`Session`] is what the box
//! believes the calculator's input should be, and [`Solution`] is what a
//! calculator answers. The box is the source of truth: a calculator is driven
//! into agreement with a session, never asked what it thinks the input was.
//!
//! Nothing here mentions Ninjabrain Bot, HTTP, JSON or a hotkey.

pub mod measurement;
pub mod session;
pub mod solution;

pub use measurement::Measurement;
pub use session::{History, Operation, Session, Throw};
pub use solution::{
    normalize_angle, Blind, Improvement, Message, Player, Prediction, Quality, Severity,
    Solution, Status, ThrowReport,
};
