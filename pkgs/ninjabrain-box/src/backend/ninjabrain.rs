//! Ninjabrain Bot as a [`Calculator`].
//!
//! The bot gets a display of its own with nothing else on it, and runs there
//! exactly as published: nothing here patches it, injects into it or wraps its
//! launcher. Every primitive is a keystroke on a hotkey it published or a
//! measurement on the clipboard it polls.

pub mod api;
pub mod hotkeys;
pub mod journal;
pub mod plan;
pub mod prefs;
pub mod xserver;

use anyhow::{anyhow, Context, Result};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::config::Config;
use crate::model::{Measurement, Operation, Session, Solution, Throw};

use super::{Calculator, Progress};

fn trace(message: impl FnOnce() -> String) {
    if std::env::var_os("NINJABRAIN_BOX_TRACE").is_some() {
        eprintln!("trace: {}", message());
    }
}
use api::Answers;
use journal::Journal;
use plan::{Planned, Planner, Primitive};
use xserver::XServer;

/// How long to give the bot to notice a measurement before giving up on it.
const PATIENCE: Duration = Duration::from_secs(3);

/// How long to leave a clipboard value alone so the bot's poll can see it. The
/// bot reads every 100ms and only reacts to a change, so anything comfortably
/// over that is enough.
const SETTLE: Duration = Duration::from_millis(180);

/// Put on the clipboard between two measurements, so that the one after it is
/// always a change even when it is identical to what was there before.
const SENTINEL: &str = "ninjabrain-box";

/// A bot process that does not outlive the overlay.
///
/// Two belts: this kills it on the way out of a clean shutdown, and the child
/// itself asks the kernel to be signalled if the overlay dies without getting
/// that far -- which it will, since the overlay is meant to be stopped with a
/// signal and Rust runs no destructors for those. The same goes for the box.
pub struct Process(Child);

impl Drop for Process {
    fn drop(&mut self) {
        xserver::stop(&mut self.0);
    }
}

/// One low-level thing to do to the box.
enum Step {
    Key(u32),
    Clipboard(String),
    /// Wait until the bot's throw list is this long.
    Await(usize),
}

pub struct Ninjabrain {
    x: Rc<XServer>,
    _bot: Process,
    answers: Arc<Mutex<Answers>>,
    solution: Solution,
    journal: Journal,
    planner: Planner,
    crosshair_correction: f64,

    /// The plan in flight: where it started, what it presses, where it aims.
    plan: Option<(Session, Vec<Planned>, Session)>,
    steps: VecDeque<Step>,
    awaiting: Option<usize>,
    ready_at: Instant,
    deadline: Instant,
}

impl Ninjabrain {
    /// Starts a display, a bot on it, and the streams back from it.
    pub fn start(
        config: &Config,
        sink: calloop::channel::Sender<Solution>,
    ) -> Result<Ninjabrain> {
        let x = Rc::new(XServer::start()?);
        let bot = start_bot(config, &x)?;
        let answers = Arc::new(Mutex::new(Answers::default()));
        api::watch(&config.bot.api, answers.clone(), sink);
        let now = Instant::now();
        Ok(Ninjabrain {
            x,
            _bot: bot,
            answers,
            solution: Solution::default(),
            journal: Journal::load(&state_directory()),
            planner: Planner::new(config.behavior.mode),
            crosshair_correction: config.bot.settings.crosshair_correction,
            plan: None,
            steps: VecDeque::new(),
            awaiting: None,
            ready_at: now,
            deadline: now,
        })
    }

    /// How many throws the bot's list has right now.
    fn throw_count(&self) -> usize {
        self.solution.throws.len()
    }

    /// Turns a plan into things to do to the box.
    ///
    /// The plan says how long the throw list is after each step, and every
    /// step that changes that length is followed by a wait for it. Working the
    /// length out here instead would get undo wrong: undo walks the bot's
    /// snapshots, and only the planner's mirror knows where that lands.
    fn expand(&self, steps: &[Planned]) -> VecDeque<Step> {
        let mut expanded = VecDeque::new();
        let mut expected = self.throw_count();
        for step in steps {
            match &step.primitive {
                Primitive::Replay(measurement) => {
                    // The bot ignores a clipboard it has already seen, and the
                    // last measurement may still be sitting there. Something
                    // else in between makes every measurement a change again.
                    expanded.push_back(Step::Clipboard(SENTINEL.to_owned()));
                    expanded.push_back(Step::Clipboard(measurement.line.clone()));
                }
                other => expanded.push_back(Step::Key(
                    other.hotkey().expect("everything but a replay is a hotkey"),
                )),
            }
            if step.throws != expected {
                expected = step.throws;
                expanded.push_back(Step::Await(expected));
            }
        }
        expanded
    }
}

impl Calculator for Ninjabrain {
    fn permits(&self, operation: Operation, session: &Session) -> Result<()> {
        self.planner.permits(operation, session)
    }

    fn direct(&mut self, from: &Session, to: &Session) -> Result<()> {
        if self.plan.is_some() {
            return Err(anyhow!("already busy with the last change"));
        }
        let primitives = self.planner.plan(from, to)?;
        if primitives.is_empty() {
            return Ok(());
        }
        self.steps = self.expand(&primitives);
        let now = Instant::now();
        self.ready_at = now;
        self.deadline = now + PATIENCE;
        self.plan = Some((from.clone(), primitives, to.clone()));
        Ok(())
    }

    fn drive(&mut self, now: Instant) -> Progress {
        if self.plan.is_none() {
            return Progress::Idle;
        }
        if now < self.ready_at {
            return Progress::Working;
        }
        if let Some(wanted) = self.awaiting {
            if self.throw_count() != wanted {
                trace(|| format!("waiting for {wanted} throws, have {}", self.throw_count()));
                if now > self.deadline {
                    self.plan = None;
                    self.steps.clear();
                    self.awaiting = None;
                    self.planner.resynchronise(&self.observed());
                    return Progress::Failed(
                        "the bot did not take a measurement back; it may be part-way \
                         through a boat angle measurement"
                            .to_owned(),
                    );
                }
                return Progress::Working;
            }
            self.awaiting = None;
        }

        while let Some(step) = self.steps.pop_front() {
            match step {
                Step::Key(keysym) => {
                    trace(|| format!("key {keysym:#x}"));
                    self.x.press(keysym)
                }
                Step::Clipboard(text) => {
                    trace(|| format!("clipboard <- {:.40}", text));
                    self.x.set_clipboard(&text);
                    self.ready_at = now + SETTLE;
                    self.deadline = now + SETTLE + PATIENCE;
                    return Progress::Working;
                }
                Step::Await(wanted) => {
                    trace(|| format!("await {wanted} (have {})", self.throw_count()));
                    self.awaiting = Some(wanted);
                    self.deadline = now + PATIENCE;
                    if self.throw_count() != wanted {
                        return Progress::Working;
                    }
                    self.awaiting = None;
                }
            }
        }

        let (from, primitives, to) = self.plan.take().expect("there was a plan");
        self.planner.executed(&from, &primitives, &to);
        Progress::Done
    }

    fn busy(&self) -> Option<&str> {
        self.plan.as_ref().map(|_| "changing the throw list")
    }

    fn solution(&self) -> &Solution {
        &self.solution
    }

    fn absorb(&mut self, solution: Solution) {
        self.solution = solution;
        // Every change the bot makes is a snapshot of its own, so noting them
        // is what lets a plan walk back through its undo.
        if self.plan.is_none() {
            let observed = self.observed();
            self.planner.note(&observed);
        }
    }

    fn observed(&self) -> Session {
        let answers = self.answers.lock().expect("the answers are not poisoned");
        Session {
            throws: answers
                .observed_throws()
                .iter()
                .map(|throw| Throw {
                    measurement: self
                        .journal
                        .measurement_for(throw, self.crosshair_correction),
                    corrections: throw.corrections,
                })
                .collect(),
        }
    }

    fn measured(&mut self, measurement: &Measurement) {
        self.journal.record(measurement);
        self.x.set_clipboard(&measurement.line);
    }

}

/// Where this backend's state lives, which is also the home the bot's JVM is
/// pointed at for its preferences.
pub fn state_directory() -> PathBuf {
    crate::config::directories().get_state_home()
}

/// Writes the preferences the next bot to start will read.
pub fn apply_settings(settings: &crate::config::Settings) -> Result<()> {
    prefs::write(&state_directory(), settings)
}

/// Starts a bot of the overlay's own, inside the box, with preferences to
/// match.
fn start_bot(config: &Config, x: &XServer) -> Result<Process> {
    let bot = std::env::var_os("NINJABRAIN_BOX_BOT")
        .context("NINJABRAIN_BOX_BOT is unset; this build is incomplete")?;

    // The bot's API port is fixed, and a second bot that cannot have it just
    // logs and carries on -- leaving the overlay reading the first one's
    // numbers and showing them as its own. Better to say so and stop.
    if std::net::TcpStream::connect(&config.bot.api).is_ok() {
        return Err(anyhow!(
            "something is already serving the Ninjabrain API on {}, \
             most likely another ninjabrain-box; stop that one first",
            config.bot.api
        ));
    }

    let state = state_directory();
    apply_settings(&config.bot.settings)?;

    let mut command = Command::new(&bot);
    command
        .env("DISPLAY", x.display())
        .env_remove("WAYLAND_DISPLAY")
        // `JDK_JAVA_OPTIONS` is read by the java launcher itself, so this
        // reaches the JVM through the wrapper nixpkgs builds without having to
        // take that wrapper apart.
        .env(
            "JDK_JAVA_OPTIONS",
            format!(
                "-Djava.util.prefs.userRoot={state} -Djava.util.prefs.systemRoot={state}",
                state = state.display(),
            ),
        );
    xserver::set_death_signal(&mut command);
    command
        .spawn()
        .map(Process)
        .with_context(|| format!("cannot start {}", PathBuf::from(bot).display()))
}
