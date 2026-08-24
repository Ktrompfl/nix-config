//! The half that owns a calculator.
//!
//! The box decides what the input should be; a [`Calculator`] works out how to
//! make its calculator agree, and whether it can at all. Everything specific
//! to one calculator lives behind this trait -- wire format, markup, hotkeys,
//! and the display it needs to run on.

pub mod ninjabrain;

use anyhow::{Result};
use std::time::Instant;

use crate::model::{Measurement, Operation, Session, Solution};

/// How much a backend may do to reach a session.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Mode {
    /// Only the calculator's published controls, driven as a person would.
    /// Nothing is synthesised, so operations that cannot be reached that way
    /// are refused rather than approximated. Run this when it matters that the
    /// tool did nothing a person at the keyboard could not have done.
    #[default]
    Restricted,
    /// Everything the backend knows how to do, which for a calculator driven
    /// by measurements means replaying ones it has already seen.
    Unbound,
}

/// How a plan in flight is getting on.
pub enum Progress {
    /// Nothing is in flight.
    Idle,
    /// Still going; call again on the next update or tick.
    Working,
    /// The calculator is where it was asked to be.
    Done,
    Failed(String),
}

pub trait Calculator {
    /// Whether `operation` could be carried out from `session`, and if not,
    /// why. Asked before doing anything, and asked again for every button the
    /// panel draws -- so the reason should read as an explanation to a person.
    fn permits(&self, operation: Operation, session: &Session) -> Result<()>;

    /// Begin moving the calculator from `from` to `to`, by the shortest route
    /// this backend can find with the primitives its mode allows.
    ///
    /// `from` is what the box believes the calculator's input to be. Returning
    /// `Ok` means a plan is now in flight; [`Calculator::drive`] carries it.
    fn direct(&mut self, from: &Session, to: &Session) -> Result<()>;

    /// Carry whatever is in flight as far as it will go without blocking.
    fn drive(&mut self, now: Instant) -> Progress;

    /// What is in flight, for the panel to say so.
    fn busy(&self) -> Option<&str>;

    /// The calculator's latest answer, translated.
    fn solution(&self) -> &Solution;

    /// A fresh answer arrived on the backend's own stream.
    fn absorb(&mut self, solution: Solution);

    /// The session the calculator's answer implies it is actually working
    /// from. Compared against the box's belief to notice divergence -- a
    /// calculator that restored its own saved state, or that someone drove by
    /// hand, is not where the box left it.
    fn observed(&self) -> Session;

    /// Hand over a measurement the game has just produced. The backend
    /// forwards it as it sees fit and remembers it if it may need to be
    /// entered again.
    fn measured(&mut self, measurement: &Measurement);
}
