//! The box itself: the session it believes in, and the calculator it drives.
//!
//! Everything the box does to a calculator happens here, and none of it needs
//! a screen. A [`Request`] goes in, a [`Session`] change comes out, and the
//! backend is asked to catch up. The overlay is a reader of this, not a part
//! of it, which is what lets the box run headless.

use anyhow::{anyhow, Context, Result};
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::action::Request;
use crate::backend::{Calculator, Progress};
use crate::model::{History, Measurement, Session, Solution};

/// How long the calculator may disagree with the box before the box concludes
/// it is not where it was left. Long enough that a measurement on its way
/// through the clipboard is not mistaken for a disagreement.
const GRACE: Duration = Duration::from_millis(1500);

/// What a request did, for whoever asked.
pub enum Answer {
    /// Nothing to report beyond having done it.
    Done,
    /// A line of output, which for [`Request::Status`] is JSON.
    Text(String),
}

pub struct Control {
    history: History,
    calculator: Box<dyn Calculator>,
    divergent_since: Option<Instant>,
    /// Set by [`Request::Quit`]; whoever is running the loop stops.
    pub exit: bool,
}

impl Control {
    pub fn new(calculator: Box<dyn Calculator>) -> Control {
        Control {
            history: History::default(),
            calculator,
            divergent_since: None,
            exit: false,
        }
    }

    pub fn session(&self) -> &Session {
        self.history.current()
    }

    pub fn solution(&self) -> &Solution {
        self.calculator.solution()
    }

    pub fn busy(&self) -> Option<&str> {
        self.calculator.busy()
    }

    /// Whether a request would do anything, which is what greys a button out.
    pub fn permits(&self, request: &Request) -> bool {
        match request {
            Request::Undo => self.history.can_undo(),
            Request::Redo => self.history.can_redo(),
            _ => match request.operation(self.session().len()) {
                Ok(Some(operation)) => self.calculator.permits(operation, self.session()).is_ok(),
                Ok(None) => true,
                Err(_) => false,
            },
        }
    }

    /// A fresh answer from the calculator.
    pub fn absorb(&mut self, solution: Solution) {
        self.calculator.absorb(solution);
        self.drive();
        self.reconcile(Instant::now());
    }

    /// Carries a plan along; called whenever nothing else is happening.
    pub fn tick(&mut self) {
        self.drive();
        self.reconcile(Instant::now());
    }

    /// Everything except what is on screen, which is the overlay's business
    /// and simply does nothing when there is no overlay.
    pub fn act(&mut self, request: &Request) -> Result<Answer> {
        match request {
            Request::Show(_) | Request::Hide(_) | Request::Toggle(_) => Ok(Answer::Done),
            Request::Quit => {
                self.exit = true;
                Ok(Answer::Done)
            }
            Request::Status => Ok(Answer::Text(self.report())),
            Request::Measure(line) => {
                self.measure(line)?;
                Ok(Answer::Done)
            }
            _ => {
                self.change(request)?;
                Ok(Answer::Text(describe(self.session())))
            }
        }
    }

    /// Takes in a line as though the game had just produced it.
    pub fn measure(&mut self, line: &str) -> Result<()> {
        let measurement = Measurement::parse(line).context("not a measurement")?;
        if self.calculator.busy().is_some() {
            return Err(anyhow!("busy changing the throw list; the measurement was not taken"));
        }
        if measurement.is_throwable() {
            let session = crate::model::Operation::Add
                .apply(self.history.current(), Some(measurement.clone()))?;
            self.history.push(session);
        }
        self.calculator.measured(&measurement);
        self.divergent_since = None;
        Ok(())
    }

    /// Moves the session, and asks the calculator to come along.
    ///
    /// Nothing moves until the calculator has agreed: a refusal leaves the box
    /// exactly where it was rather than throwing the history away.
    fn change(&mut self, request: &Request) -> Result<()> {
        let from = self.calculator.observed();
        let (target, commit) = match request {
            Request::Undo => (self.history.peek_undo()?.clone(), Commit::Undo),
            Request::Redo => (self.history.peek_redo()?.clone(), Commit::Redo),
            other => {
                let current = self.history.current();
                let operation = other
                    .operation(current.len())?
                    .with_context(|| format!("{} changes nothing", other.encode()))?;
                self.calculator
                    .permits(operation, current)
                    .with_context(|| operation.describe())?;
                let next = operation.apply(current, None)?;
                (next.clone(), Commit::Push(next))
            }
        };
        self.calculator.direct(&from, &target)?;
        match commit {
            Commit::Undo => self.history.undo(),
            Commit::Redo => self.history.redo(),
            Commit::Push(session) => self.history.push(session),
        }
        Ok(())
    }

    fn drive(&mut self) {
        match self.calculator.drive(Instant::now()) {
            Progress::Idle | Progress::Working => {}
            Progress::Done => self.divergent_since = None,
            Progress::Failed(reason) => {
                eprintln!("ninjabrain-box: {reason}");
                self.history.restart(self.calculator.observed());
                self.divergent_since = None;
            }
        }
    }

    /// Notices when the calculator is not where the box left it -- it restored
    /// its own saved state, or somebody drove it by hand -- and adopts its
    /// version rather than fighting it.
    fn reconcile(&mut self, now: Instant) {
        if self.calculator.busy().is_some() {
            self.divergent_since = None;
            return;
        }
        let observed = self.calculator.observed();
        if observed.same_as(self.history.current()) {
            self.divergent_since = None;
            return;
        }
        match self.divergent_since {
            None => self.divergent_since = Some(now),
            Some(since) if now.duration_since(since) < GRACE => {}
            Some(_) => {
                trace(|| {
                    format!(
                        "adopting the calculator: box has {}, calculator has {}",
                        describe(self.history.current()),
                        describe(&observed)
                    )
                });
                self.history.restart(observed);
                self.divergent_since = None;
            }
        }
    }

    fn report(&self) -> String {
        let solution = self.calculator.solution();
        let report = Report {
            throws: self
                .session()
                .throws
                .iter()
                .map(|throw| ThrowReport {
                    x: throw.measurement.as_ref().map(|m| m.x),
                    z: throw.measurement.as_ref().map(|m| m.z),
                    corrections: throw.corrections,
                    replayable: throw.is_replayable(),
                })
                .collect(),
            predictions: solution
                .predictions
                .iter()
                .map(|prediction| PredictionReport {
                    chunk: [prediction.chunk.0, prediction.chunk.1],
                    certainty: prediction.certainty,
                })
                .collect(),
            messages: solution
                .messages
                .iter()
                .map(|message| message.text.clone())
                .collect(),
            blind: solution.blind.as_ref().map(|blind| blind.quality.label()),
            busy: self.calculator.busy().map(str::to_owned),
            can_undo: self.history.can_undo(),
            can_redo: self.history.can_redo(),
        };
        serde_json::to_string(&report).unwrap_or_else(|error| format!("{{\"error\":\"{error}\"}}"))
    }
}

enum Commit {
    Undo,
    Redo,
    Push(Session),
}

#[derive(Serialize)]
struct Report {
    throws: Vec<ThrowReport>,
    predictions: Vec<PredictionReport>,
    messages: Vec<String>,
    blind: Option<&'static str>,
    busy: Option<String>,
    can_undo: bool,
    can_redo: bool,
}

#[derive(Serialize)]
struct ThrowReport {
    x: Option<f64>,
    z: Option<f64>,
    corrections: i32,
    replayable: bool,
}

#[derive(Serialize)]
struct PredictionReport {
    chunk: [i32; 2],
    certainty: f64,
}

/// A session in one line, for answers and for tracing.
fn describe(session: &Session) -> String {
    let throws: Vec<String> = session
        .throws
        .iter()
        .map(|throw| {
            let where_from = throw
                .measurement
                .as_ref()
                .map_or("?".to_owned(), |m| format!("{:.0}", m.x));
            format!("{where_from}{:+}", throw.corrections)
        })
        .collect();
    format!("[{}]", throws.join(" "))
}

/// `NINJABRAIN_BOX_TRACE=1` narrates the box's disagreements with the
/// calculator. A stream of these is always a bug in the box.
fn trace(message: impl FnOnce() -> String) {
    if std::env::var_os("NINJABRAIN_BOX_TRACE").is_some() {
        eprintln!("ninjabrain-box: {}", message());
    }
}
