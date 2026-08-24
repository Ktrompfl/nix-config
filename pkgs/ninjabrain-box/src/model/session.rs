//! What the box believes the calculator's input should be, and how it got there.
//!
//! Every [`Operation`] is a function from one [`Session`] to the next, and
//! [`History`] records the line of them. That is what makes undo work across
//! operations the calculator has never heard of: undo asks the calculator to
//! undo nothing, it moves a cursor and leaves the backend to work out the way
//! back.

use anyhow::{anyhow, Context, Result};
use super::Measurement;

/// One throw, as the box remembers making it.
#[derive(Clone, Debug)]
pub struct Throw {
    /// What the game said. `None` for a throw the box did not see made --
    /// one the calculator restored from its own saved state, say. Such a
    /// throw can be kept and adjusted, but never re-entered.
    pub measurement: Option<Measurement>,
    /// Adjustment steps applied since, positive or negative.
    pub corrections: i32,
}

impl Throw {
    pub fn new(measurement: Measurement) -> Throw {
        Throw {
            measurement: Some(measurement),
            corrections: 0,
        }
    }

    /// Whether this throw could be entered again from scratch.
    pub fn is_replayable(&self) -> bool {
        self.measurement.is_some()
    }
}

impl Throw {
    fn same_as(&self, other: &Throw) -> bool {
        self.corrections == other.corrections
            && match (&self.measurement, &other.measurement) {
                (Some(a), Some(b)) => a.is(b),
                (None, None) => true,
                _ => false,
            }
    }
}

/// The input a calculator should be working from.
#[derive(Clone, Debug, Default)]
pub struct Session {
    pub throws: Vec<Throw>,
}

impl Session {
    pub fn len(&self) -> usize {
        self.throws.len()
    }

    pub fn same_as(&self, other: &Session) -> bool {
        self.throws.len() == other.throws.len()
            && self
                .throws
                .iter()
                .zip(&other.throws)
                .all(|(a, b)| a.same_as(b))
    }

    /// Whether every throw could be entered again, which is what a rebuild
    /// needs. Names the first one that could not, for the message.
    pub fn unreplayable(&self) -> Option<usize> {
        self.throws
            .iter()
            .position(|throw| !throw.is_replayable())
            .map(|index| index + 1)
    }
}

/// The operations the box understands. Each is a function from one session to
/// the next; none of them says anything about how a calculator is to be made
/// to agree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    /// A throw the game just produced.
    Add,
    /// Throw the whole session away.
    Reset,
    /// Remove one throw, counting from one.
    Drop(usize),
    /// Move one throw's adjustment, counting from one.
    Adjust(usize, i32),
}

impl Operation {
    /// The name shown when an operation cannot be done.
    pub fn describe(self) -> String {
        match self {
            Operation::Add => "adding a throw".to_owned(),
            Operation::Reset => "resetting".to_owned(),
            Operation::Drop(index) => format!("dropping throw {index}"),
            Operation::Adjust(index, by) => format!("moving throw {index} by {by:+}"),
        }
    }

    /// Applies this operation, or says why it does not apply. `measurement` is
    /// only used by [`Operation::Add`].
    pub fn apply(
        self,
        session: &Session,
        measurement: Option<Measurement>,
    ) -> Result<Session> {
        let mut next = session.clone();
        let throw_at = |index: usize| -> Result<usize> {
            match index {
                0 => Err(anyhow!("throws are numbered from 1")),
                _ if index > session.len() => Err(match session.len() {
                    0 => anyhow!("there are no throws"),
                    count => anyhow!("there is no throw {index}; there are {count}"),
                }),
                _ => Ok(index - 1),
            }
        };
        match self {
            Operation::Add => {
                let measurement = measurement.context("no measurement to add")?;
                // A measurement identical to the last throw is the same
                // observation, not a second throw. Calculators take that view
                // too -- the box has to agree with them, or it books a throw
                // the calculator never made. It happens in practice: a
                // clipboard can be offered to a watcher more than once for one
                // copy, and a player standing still can press F3+C twice.
                let repeat = next
                    .throws
                    .last()
                    .and_then(|throw| throw.measurement.as_ref())
                    .is_some_and(|last| last.is(&measurement));
                if !repeat {
                    next.throws.push(Throw::new(measurement));
                }
            }
            Operation::Reset => next.throws.clear(),
            Operation::Drop(index) => {
                next.throws.remove(throw_at(index)?);
            }
            Operation::Adjust(index, by) => {
                let throw = &mut next.throws[throw_at(index)?];
                throw.corrections = throw.corrections.saturating_add(by);
            }
        }
        Ok(next)
    }
}

/// Every session the box has been in, and where in that line it is now.
///
/// Undo and redo move the cursor. They are not passed to the calculator --
/// the calculator's own undo stack is its business, and would know nothing
/// about an operation the box invented.
pub struct History {
    line: Vec<Session>,
    cursor: usize,
}

/// Deep enough for any session; a stronghold is found in a dozen throws or
/// never, and every one of them is a handful of operations.
const DEPTH: usize = 256;

impl Default for History {
    fn default() -> History {
        History {
            line: vec![Session::default()],
            cursor: 0,
        }
    }
}

impl History {
    pub fn current(&self) -> &Session {
        &self.line[self.cursor]
    }

    /// Records a new session, dropping anything that had been undone past it.
    pub fn push(&mut self, session: Session) {
        if self.current().same_as(&session) {
            return;
        }
        self.line.truncate(self.cursor + 1);
        self.line.push(session);
        let excess = self.line.len().saturating_sub(DEPTH);
        self.line.drain(..excess);
        self.cursor = self.line.len() - 1;
    }

    pub fn can_undo(&self) -> bool {
        self.cursor > 0
    }

    pub fn can_redo(&self) -> bool {
        self.cursor + 1 < self.line.len()
    }

    /// Where undo would land, without going there. The cursor only moves once
    /// something has agreed to take the calculator with it.
    pub fn peek_undo(&self) -> Result<&Session> {
        if !self.can_undo() {
            return Err(anyhow!("nothing to undo"));
        }
        Ok(&self.line[self.cursor - 1])
    }

    pub fn peek_redo(&self) -> Result<&Session> {
        if !self.can_redo() {
            return Err(anyhow!("nothing to redo"));
        }
        Ok(&self.line[self.cursor + 1])
    }

    pub fn undo(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    pub fn redo(&mut self) {
        self.cursor = (self.cursor + 1).min(self.line.len() - 1);
    }

    /// Throws the line away and starts again from `session`. Used when the
    /// calculator turns out to be somewhere the box did not put it, which
    /// makes every remembered session a fiction.
    pub fn restart(&mut self, session: Session) {
        self.line = vec![session];
        self.cursor = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Measurement;

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

    #[test]
    fn operations_are_functions_on_a_session() {
        let start = session(&[1.0, 2.0, 3.0]);
        let dropped = Operation::Drop(2).apply(&start, None).expect("drops");
        assert_eq!(dropped.len(), 2);
        assert!(dropped.throws[1].measurement.as_ref().unwrap().is(&measurement(3.0)));

        let adjusted = Operation::Adjust(1, 2).apply(&start, None).expect("adjusts");
        assert_eq!(adjusted.throws[0].corrections, 2);
        assert_eq!(adjusted.throws[1].corrections, 0);

        assert_eq!(Operation::Reset.apply(&start, None).expect("resets").len(), 0);
    }

    /// One observation is one throw, however many times it arrives.
    #[test]
    fn the_same_measurement_twice_is_one_throw() {
        let start = session(&[1.0]);
        let again = Operation::Add
            .apply(&start, Some(measurement(1.0)))
            .expect("adds");
        assert_eq!(again.len(), 1);

        // A different one is still a throw, and so is the first one coming
        // back round after it.
        let two = Operation::Add
            .apply(&start, Some(measurement(2.0)))
            .expect("adds");
        assert_eq!(two.len(), 2);
        let three = Operation::Add
            .apply(&two, Some(measurement(1.0)))
            .expect("adds");
        assert_eq!(three.len(), 3);
    }

    #[test]
    fn refuses_throws_that_are_not_there() {
        let start = session(&[1.0]);
        assert!(Operation::Drop(0).apply(&start, None).is_err());
        assert!(Operation::Drop(2).apply(&start, None).is_err());
        assert!(Operation::Drop(1).apply(&start, None).is_ok());
    }

    #[test]
    fn undo_walks_back_through_operations_the_calculator_never_saw() {
        let mut history = History::default();
        let one = session(&[1.0]);
        let two = session(&[1.0, 2.0]);
        history.push(one.clone());
        history.push(two.clone());
        let dropped = Operation::Drop(1).apply(&two, None).expect("drops");
        history.push(dropped.clone());

        assert!(history.current().same_as(&dropped));
        assert!(history.peek_undo().expect("can undo").same_as(&two));
        history.undo();
        assert!(history.current().same_as(&two));
        history.undo();
        assert!(history.current().same_as(&one));
        history.redo();
        assert!(history.current().same_as(&two));
    }

    /// Doing something new after an undo abandons what was undone.
    #[test]
    fn a_new_operation_drops_the_future() {
        let mut history = History::default();
        history.push(session(&[1.0]));
        history.push(session(&[1.0, 2.0]));
        history.undo();
        assert!(history.can_redo());
        history.push(session(&[1.0, 3.0]));
        assert!(!history.can_redo());
        history.undo();
        assert!(history.current().same_as(&session(&[1.0])));
    }

}
