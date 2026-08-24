//! Compiling a session change into things the bot can be told to do.
//!
//! [`Primitive`] is the whole of what the bot offers, so any plan is a
//! sequence a person could have performed. Restricted mode drops
//! [`Primitive::Replay`], and that is the only difference between the modes:
//! without it nothing can be inserted, so no throw but the last is reachable.
//!
//! Every primitive except undo and redo either clears the session or appends
//! to it, and undo and redo walk the bot's snapshot list. An optimal plan is
//! therefore always a navigation followed by a build, so [`Planner::plan`]
//! enumerates the few places it could start from -- stay, reset, or walk `k`
//! snapshots either way -- and takes the cheapest. That is exhaustive over
//! plans of that shape, which is all of them.

use anyhow::{Context, Result};
use crate::backend::Mode;
use crate::model::{Measurement, Operation, Session};

/// One thing to do, and how long the bot's throw list will be once it is done.
///
/// The length has to come from here rather than be re-derived by the caller:
/// undo and redo change it by walking the snapshot list, and the mirror is the
/// only thing that knows where that lands.
#[derive(Clone, Debug, PartialEq)]
pub struct Planned {
    pub primitive: Primitive,
    pub throws: usize,
}

/// One thing the bot can be told to do.
#[derive(Clone, Debug, PartialEq)]
pub enum Primitive {
    Reset,
    Undo,
    Redo,
    Increment,
    Decrement,
    /// Put this measurement back on the clipboard, appending a throw.
    Replay(Measurement),
}

impl Primitive {
    /// The bot hotkey this is, if it is one.
    pub fn hotkey(&self) -> Option<u32> {
        let name = match self {
            Primitive::Reset => "reset",
            Primitive::Undo => "undo",
            Primitive::Redo => "redo",
            Primitive::Increment => "increment",
            Primitive::Decrement => "decrement",
            Primitive::Replay(_) => return None,
        };
        Some(super::hotkeys::get(name).keysym)
    }
}

/// The throw list as the planner compares it: everything that distinguishes
/// one session's input from another, and nothing that does not.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Shape(Vec<Slot>);

#[derive(Clone, Debug, PartialEq, Eq)]
struct Slot {
    /// `None` for a throw whose measurement the box never saw.
    measurement: Option<Measurement>,
    corrections: i32,
    replayable: bool,
}

impl Shape {
    fn of(session: &Session) -> Shape {
        Shape(
            session
                .throws
                .iter()
                .map(|throw| Slot {
                    measurement: throw.measurement.clone(),
                    corrections: throw.corrections,
                    replayable: throw.is_replayable(),
                })
                .collect(),
        )
    }
}

/// The bot's snapshot list, as far as this backend has driven it.
///
/// The bot snapshots its *entire* model per change, so undoing also reverts a
/// boat angle or a lock. Only snapshots this backend caused are recorded here,
/// and undo is only ever aimed at those.
pub struct Mirror {
    line: Vec<Shape>,
    cursor: usize,
}

impl Mirror {
    fn restart(shape: Shape) -> Mirror {
        Mirror {
            line: vec![shape],
            cursor: 0,
        }
    }

    /// The bot deduplicates snapshots, so a change that lands on the state
    /// already at the cursor adds nothing to undo through.
    fn record(&mut self, shape: Shape) {
        if self.line[self.cursor] == shape {
            return;
        }
        self.line.truncate(self.cursor + 1);
        self.line.push(shape);
        self.cursor = self.line.len() - 1;
    }
}

/// How much a step costs, in things the user waits for. A replayed
/// measurement has to be noticed by a clipboard poll, so it is worth several
/// keystrokes.
const REPLAY_COST: usize = 6;
const KEY_COST: usize = 1;

/// Works out what to press.
pub struct Planner {
    mode: Mode,
    mirror: Option<Mirror>,
}

impl Planner {
    pub fn new(mode: Mode) -> Planner {
        Planner { mode, mirror: None }
    }

    /// Forgets what the bot's undo stack looked like. Called whenever the bot
    /// turns out to be somewhere the box did not put it.
    pub fn resynchronise(&mut self, session: &Session) {
        self.mirror = Some(Mirror::restart(Shape::of(session)));
    }

    /// Whether `operation` could be carried out from `session`, and if not,
    /// why. This plans it and throws the plan away rather than reasoning about
    /// what a mode can reach, so what a button offers is exactly what pressing
    /// it would do.
    pub fn permits(&self, operation: Operation, session: &Session) -> Result<()> {
        let after = operation.apply(session, None)?;
        self.plan(session, &after).map(drop)
    }

    /// The shortest sequence of primitives taking the bot from `from` to `to`.
    pub fn plan(&self, from: &Session, to: &Session) -> Result<Vec<Planned>> {
        if from.same_as(to) {
            return Ok(Vec::new());
        }
        let start = Shape::of(from);
        let target = Shape::of(to);

        let mirror = match self.mirror.as_ref() {
            Some(mirror) if mirror.line[mirror.cursor] == start => Some(mirror),
            // The mirror does not describe where the bot is, so undo and redo
            // would be walking into the unknown.
            _ => None,
        };

        let mut best: Option<(usize, Vec<Primitive>)> = None;
        let mut consider = |lead: Vec<Primitive>, from: &Shape| {
            let Some(build) = self.build(from, &target) else {
                return;
            };
            let cost = cost_of(&lead) + cost_of(&build);
            if best.as_ref().is_none_or(|(best, _)| cost < *best) {
                let mut steps = lead;
                steps.extend(build);
                best = Some((cost, steps));
            }
        };

        // Stay where we are, or clear the decks.
        consider(Vec::new(), &start);
        consider(vec![Primitive::Reset], &Shape(Vec::new()));

        // Or walk the bot's own snapshots, in either direction.
        if let Some(mirror) = mirror {
            for back in 1..=mirror.cursor {
                let lead = vec![Primitive::Undo; back];
                consider(lead, &mirror.line[mirror.cursor - back].clone());
            }
            for forward in 1..mirror.line.len() - mirror.cursor {
                let lead = vec![Primitive::Redo; forward];
                consider(lead, &mirror.line[mirror.cursor + forward].clone());
            }
        }

        let (_, steps) = best.with_context(|| self.refusal(from, to))?;
        Ok(annotate(&start, mirror, steps))
    }

    /// The forward-only build from `from` to `to`, if there is one.
    ///
    /// Forward means append and adjust-the-last, which is all the bot offers
    /// without putting a measurement back. So `from` has to be a prefix of
    /// `to` already, give or take the adjustment on its own last throw.
    fn build(&self, from: &Shape, to: &Shape) -> Option<Vec<Primitive>> {
        if from.0.len() > to.0.len() {
            return None;
        }
        // Everything but the last of `from` has to match outright: there is no
        // reaching back past the end of the list.
        let settled = from.0.len().saturating_sub(1);
        if from.0[..settled] != to.0[..settled] {
            return None;
        }

        let mut steps = Vec::new();
        // The last throw of `from`, if any, may still be adjusted into place.
        if let Some(last) = from.0.len().checked_sub(1) {
            let (have, want) = (&from.0[last], &to.0[last]);
            if have.measurement != want.measurement {
                return None;
            }
            steps.extend(adjust(have.corrections, want.corrections));
        }

        for slot in &to.0[from.0.len()..] {
            if self.mode == Mode::Restricted {
                return None;
            }
            let measurement = slot.measurement.clone().filter(|_| slot.replayable)?;
            steps.push(Primitive::Replay(measurement));
            steps.extend(adjust(0, slot.corrections));
        }
        Some(steps)
    }

    /// Why nothing worked, said usefully.
    fn refusal(&self, from: &Session, to: &Session) -> String {
        if self.mode == Mode::Restricted {
            return format!(
                "in restricted mode the bot's controls cannot get from {} throw{} to that list: \
                 nothing but the last throw can be changed, and no throw can be put back. \
                 Set mode = \"unbound\" to allow replaying measurements.",
                from.len(),
                if from.len() == 1 { "" } else { "s" }
            );
        }
        match to.unreplayable() {
            Some(index) => format!(
                "throw {index} cannot be entered again -- it was made with a boat angle, or the \
                 box never saw the measurement it was made from"
            ),
            None => "there is no way to get there from here".to_owned(),
        }
    }

    /// Notes a state the bot has reached on its own -- a measurement it just
    /// took -- which it will have snapshotted, and which undo can therefore
    /// reach.
    pub fn note(&mut self, session: &Session) {
        let shape = Shape::of(session);
        match self.mirror.as_mut() {
            Some(mirror) => mirror.record(shape),
            None => self.mirror = Some(Mirror::restart(shape)),
        }
    }

    /// Notes that a plan has been carried out, so that undo and redo keep
    /// meaning what they mean.
    ///
    /// The bot takes a snapshot per change, so every intermediate state of the
    /// plan is somewhere its undo can reach -- and those are the only places
    /// this backend will ever aim undo at.
    pub fn executed(&mut self, from: &Session, primitives: &[Planned], to: &Session) {
        let mut shape = Shape::of(from);
        let target = Shape::of(to);
        let mirror = self
            .mirror
            .get_or_insert_with(|| Mirror::restart(shape.clone()));
        if mirror.line[mirror.cursor] != shape {
            *mirror = Mirror::restart(shape.clone());
        }
        for primitive in primitives.iter().map(|planned| &planned.primitive) {
            match primitive {
                Primitive::Undo => mirror.cursor = mirror.cursor.saturating_sub(1),
                Primitive::Redo => {
                    mirror.cursor = (mirror.cursor + 1).min(mirror.line.len().saturating_sub(1))
                }
                other => {
                    apply(&mut shape, other);
                    mirror.record(shape.clone());
                    continue;
                }
            }
            shape = mirror.line[mirror.cursor].clone();
        }
        // The plan is what it is; if walking it did not land where it was
        // aimed, the mirror is a fiction and is better thrown away.
        if mirror.line[mirror.cursor] != target {
            *mirror = Mirror::restart(target);
        }
    }
}

/// Walks a plan, working out how long the throw list is after each step.
fn annotate(start: &Shape, mirror: Option<&Mirror>, steps: Vec<Primitive>) -> Vec<Planned> {
    let mut shape = start.clone();
    let mut cursor = mirror.map_or(0, |mirror| mirror.cursor);
    steps
        .into_iter()
        .map(|primitive| {
            match (&primitive, mirror) {
                (Primitive::Undo, Some(mirror)) => {
                    cursor = cursor.saturating_sub(1);
                    shape = mirror.line[cursor].clone();
                }
                (Primitive::Redo, Some(mirror)) => {
                    cursor = (cursor + 1).min(mirror.line.len() - 1);
                    shape = mirror.line[cursor].clone();
                }
                // Without a mirror there is no undo in any plan, so this is
                // only ever the forward primitives.
                (other, _) => apply(&mut shape, other),
            }
            Planned {
                primitive,
                throws: shape.0.len(),
            }
        })
        .collect()
}

/// What a forward primitive does to the throw list.
fn apply(shape: &mut Shape, primitive: &Primitive) {
    match primitive {
        Primitive::Reset => shape.0.clear(),
        Primitive::Replay(measurement) => shape.0.push(Slot {
            measurement: Some(measurement.clone()),
            corrections: 0,
            replayable: true,
        }),
        Primitive::Increment | Primitive::Decrement => {
            let Some(last) = shape.0.last_mut() else {
                return;
            };
            match primitive {
                Primitive::Increment => last.corrections += 1,
                _ => last.corrections -= 1,
            }
        }
        Primitive::Undo | Primitive::Redo => {}
    }
}

/// The presses that take one throw's adjustment from `have` to `want`.
fn adjust(have: i32, want: i32) -> Vec<Primitive> {
    let mut steps = Vec::new();
    let delta = want - have;
    let key = if delta > 0 {
        Primitive::Increment
    } else {
        Primitive::Decrement
    };
    for _ in 0..delta.unsigned_abs() {
        steps.push(key.clone());
    }
    steps
}

fn cost_of(steps: &[Primitive]) -> usize {
    steps
        .iter()
        .map(|step| match step {
            Primitive::Replay(_) => REPLAY_COST,
            _ => KEY_COST,
        })
        .sum()
}



#[cfg(test)]
mod tests {
    use super::*;

    use crate::model::Throw;

    /// Most tests care what gets pressed, not how long the list is after each
    /// press, so they read the plan as bare primitives.
    fn primitives(planned: &[Planned]) -> Vec<Primitive> {
        planned.iter().map(|step| step.primitive.clone()).collect()
    }

    fn measurement(x: f64) -> Measurement {
        Measurement::parse(&format!(
            "/execute in minecraft:overworld run tp @s {x:.2} 68.00 -20.25 -45.60 -31.00"
        ))
        .expect("a measurement")
    }

    fn session(xs: &[f64]) -> Session {
        Session {
            throws: xs.iter().map(|x| Throw::new(measurement(*x))).collect(),
        }
    }

    /// Adjusting the last throw is what the bot's hotkeys already do, so the
    /// plan should be exactly those presses -- in either mode.
    #[test]
    fn adjusting_the_last_throw_is_just_key_presses() {
        for mode in [Mode::Restricted, Mode::Unbound] {
            let from = session(&[1.0, 2.0]);
            let to = Operation::Adjust(2, 3).apply(&from, None).expect("adjusts");
            let steps = primitives(&Planner::new(mode).plan(&from, &to).expect("a plan"));
            assert_eq!(steps, vec![Primitive::Increment; 3], "{mode:?}");
        }
    }

    #[test]
    fn appending_does_not_disturb_what_is_there() {
        let from = session(&[1.0]);
        let to = session(&[1.0, 2.0]);
        let steps = primitives(&Planner::new(Mode::Unbound).plan(&from, &to).expect("a plan"));
        assert_eq!(steps, vec![Primitive::Replay(measurement(2.0))]);
    }

    /// A row offers what is possible, not what a mode can reach in general:
    /// the last throw can be dropped even in restricted mode, because the
    /// bot's own undo goes back to the state before it.
    #[test]
    fn the_last_throw_can_be_dropped_once_undo_reaches_it() {
        let one = session(&[1.0]);
        let two = session(&[1.0, 2.0]);
        let mut planner = Planner::new(Mode::Restricted);

        // Before the box knows what the bot has snapshotted, it cannot say
        // undo would work, so it does not offer it.
        assert!(planner.permits(Operation::Drop(2), &two).is_err());

        planner.note(&Session::default());
        planner.note(&one);
        planner.note(&two);
        assert!(planner.permits(Operation::Drop(2), &two).is_ok());
        assert_eq!(
            primitives(&planner.plan(&two, &one).expect("a plan")),
            vec![Primitive::Undo]
        );
        // An earlier throw is still out of reach: undo cannot get there
        // without losing the one after it.
        assert!(planner.permits(Operation::Drop(1), &two).is_err());
    }

    /// Nothing but the last throw is reachable without putting measurements
    /// back, so restricted mode refuses rather than approximating.
    #[test]
    fn restricted_mode_cannot_reach_an_earlier_throw() {
        let from = session(&[1.0, 2.0, 3.0]);
        let to = Operation::Drop(2).apply(&from, None).expect("drops");
        let error = Planner::new(Mode::Restricted)
            .plan(&from, &to)
            .expect_err("refused")
            .to_string();
        assert!(error.contains("restricted mode"), "{error}");

        let planner = Planner::new(Mode::Restricted);
        assert!(planner.permits(Operation::Drop(2), &from).is_err());
        assert!(planner.permits(Operation::Adjust(1, 1), &from).is_err());
        // The last throw is still fair game, and so is starting over.
        assert!(planner.permits(Operation::Adjust(3, 1), &from).is_ok());
        assert!(planner.permits(Operation::Reset, &from).is_ok());
    }

    #[test]
    fn unbound_mode_rebuilds_what_it_cannot_reach() {
        let from = session(&[1.0, 2.0, 3.0]);
        let to = Operation::Drop(2).apply(&from, None).expect("drops");
        let steps = primitives(&Planner::new(Mode::Unbound).plan(&from, &to).expect("a plan"));
        assert_eq!(
            steps,
            vec![
                Primitive::Reset,
                Primitive::Replay(measurement(1.0)),
                Primitive::Replay(measurement(3.0)),
            ]
        );
    }

    /// A rebuild has to put every adjustment back too, not just the throws.
    #[test]
    fn a_rebuild_restores_corrections_and_alt_std() {
        let mut from = session(&[1.0, 2.0, 3.0]);
        from.throws[0].corrections = 2;
        let to = Operation::Drop(2).apply(&from, None).expect("drops");
        let steps = primitives(&Planner::new(Mode::Unbound).plan(&from, &to).expect("a plan"));
        assert_eq!(
            steps,
            vec![
                Primitive::Reset,
                Primitive::Replay(measurement(1.0)),
                Primitive::Increment,
                Primitive::Increment,
                Primitive::Replay(measurement(3.0)),
            ]
        );
    }

    /// Walking back through the bot's own snapshots beats rebuilding when the
    /// bot put those snapshots there.
    #[test]
    fn undo_is_cheaper_than_a_rebuild_when_it_reaches() {
        let one = session(&[1.0]);
        let two = session(&[1.0, 2.0]);
        let mut planner = Planner::new(Mode::Unbound);
        planner.executed(
            &Session::default(),
            &[
                Planned { primitive: Primitive::Replay(measurement(1.0)), throws: 1 },
                Planned { primitive: Primitive::Replay(measurement(2.0)), throws: 2 },
            ],
            &two,
        );

        // Back to one throw: one undo, rather than reset and a replay.
        let planned = planner.plan(&two, &one).expect("a plan");
        assert_eq!(primitives(&planned), vec![Primitive::Undo]);
        // And it knows the list is one throw shorter afterwards -- which only
        // the mirror can say, and which the caller has to wait for.
        assert_eq!(planned[0].throws, 1);

        // And with no mirror there is nothing to undo through, so the same
        // change costs a rebuild.
        let steps = primitives(&Planner::new(Mode::Unbound).plan(&two, &one).expect("a plan"));
        assert_eq!(
            steps,
            vec![Primitive::Reset, Primitive::Replay(measurement(1.0))]
        );
    }

    /// A throw the box never saw made cannot be put back, so a rebuild that
    /// would need to is refused rather than quietly dropping it.
    #[test]
    fn refuses_to_rebuild_what_it_cannot_reproduce() {
        let mut from = session(&[1.0, 2.0, 3.0]);
        from.throws[0].measurement = None;
        let to = Operation::Drop(2).apply(&from, None).expect("drops");
        let error = Planner::new(Mode::Unbound)
            .plan(&from, &to)
            .expect_err("refused")
            .to_string();
        assert!(error.contains("cannot be entered again"), "{error}");

        // A boat throw is unreproducible for the same reason.
    }
}
