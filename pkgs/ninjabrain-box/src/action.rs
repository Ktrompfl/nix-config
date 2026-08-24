//! What the box can be asked to do.
//!
//! One request is one line on the socket, so [`Request::encode`] and
//! [`Request::parse`] are inverses and the command line is a thin wrapper over
//! them.

use anyhow::{anyhow, Context, Result};
use crate::model::Operation;

/// A throw, counted from one at the front or from minus one at the back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Index(pub i32);

impl Index {
    pub const LAST: Index = Index(-1);

    /// Turns this into a throw number counting from one, given how many
    /// throws there are.
    pub fn resolve(self, throws: usize) -> Result<usize> {
        let throws = throws as i32;
        let number = match self.0 {
            0 => return Err(anyhow!("throws are numbered from 1, or from -1 backwards")),
            n if n < 0 => throws + 1 + n,
            n => n,
        };
        if number < 1 || number > throws {
            return Err(match throws {
                0 => anyhow!("there are no throws"),
                1 => anyhow!("there is no throw {}; there is 1", self.0),
                _ => anyhow!("there is no throw {}; there are {throws}", self.0),
            });
        }
        Ok(number as usize)
    }
}

impl std::str::FromStr for Index {
    type Err = anyhow::Error;

    fn from_str(text: &str) -> Result<Index> {
        match text.parse() {
            Ok(0) | Err(_) => Err(anyhow!("a throw number, counting from 1 or from -1 back")),
            Ok(number) => Ok(Index(number)),
        }
    }
}

/// What a visibility request applies to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, clap::ValueEnum)]
#[clap(rename_all = "lower")]
pub enum Target {
    /// The window itself.
    #[default]
    All,
    /// The eye throw table within it.
    Throws,
}

impl Target {
    fn name(self) -> &'static str {
        match self {
            Target::All => "all",
            Target::Throws => "throws",
        }
    }

    fn parse(name: &str) -> Option<Target> {
        match name {
            "all" => Some(Target::All),
            "throws" => Some(Target::Throws),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Request {
    Show(Target),
    Hide(Target),
    Toggle(Target),
    Quit,

    Reset,
    Undo,
    Redo,
    Drop(Index),
    /// Move a throw's angle by a number of adjustment steps.
    Adjust(Index, i32),
    /// Feed the box a line as if the game had just put it on the clipboard.
    ///
    /// This is the input the box normally takes from the compositor, offered
    /// directly so the whole of it can be driven without one.
    Measure(String),
    /// Report what the box and the calculator think, as JSON.
    Status,
}

impl Request {
    pub fn encode(&self) -> String {
        match self {
            Request::Show(target) => format!("show {}", target.name()),
            Request::Hide(target) => format!("hide {}", target.name()),
            Request::Toggle(target) => format!("toggle {}", target.name()),
            Request::Quit => "quit".to_owned(),
            Request::Reset => "reset".to_owned(),
            Request::Undo => "undo".to_owned(),
            Request::Redo => "redo".to_owned(),
            Request::Drop(index) => format!("drop {}", index.0),
            Request::Adjust(index, steps) => format!("adjust {} {steps}", index.0),
            Request::Measure(line) => format!("measure {line}"),
            Request::Status => "status".to_owned(),
        }
    }

    pub fn parse(line: &str) -> Result<Request> {
        let (head, rest) = match line.trim().split_once(' ') {
            Some((head, rest)) => (head, rest.trim()),
            None => (line.trim(), ""),
        };
        let target = || match rest {
            "" => Ok(Target::default()),
            name => Target::parse(name).with_context(|| format!("no such target: {name}")),
        };
        let index = || match rest {
            "" => Ok(Index::LAST),
            text => text.parse(),
        };
        let alone = |request| match rest {
            "" => Ok(request),
            extra => Err(anyhow!("unexpected {extra:?} after {head}")),
        };
        match head {
            "show" => Ok(Request::Show(target()?)),
            "hide" => Ok(Request::Hide(target()?)),
            "toggle" => Ok(Request::Toggle(target()?)),
            "quit" => alone(Request::Quit),
            "reset" => alone(Request::Reset),
            "undo" => alone(Request::Undo),
            "redo" => alone(Request::Redo),
            "status" => alone(Request::Status),
            "drop" => Ok(Request::Drop(index()?)),
            "adjust" => parse_adjust(rest),
            "measure" if !rest.is_empty() => Ok(Request::Measure(rest.to_owned())),
            "measure" => Err(anyhow!("measure needs a line to feed in")),
            _ => Err(anyhow!("no such request: {head}")),
        }
    }

    /// The session operation this asks for, if it asks for one.
    pub fn operation(&self, throws: usize) -> Result<Option<Operation>> {
        Ok(Some(match self {
            Request::Reset => Operation::Reset,
            Request::Drop(index) => Operation::Drop(index.resolve(throws)?),
            Request::Adjust(index, by) => Operation::Adjust(index.resolve(throws)?, *by),
            _ => return Ok(None),
        }))
    }
}

/// `adjust [THROW] STEPS`, where a single argument is the step count and the
/// throw defaults to the last.
fn parse_adjust(rest: &str) -> Result<Request> {
    let steps = |text: &str| {
        text.parse::<i32>()
            .context("a whole number of steps")
    };
    let mut words = rest.split_whitespace();
    match (words.next(), words.next(), words.next()) {
        (Some(only), None, _) => Ok(Request::Adjust(Index::LAST, steps(only)?)),
        (Some(index), Some(amount), None) => Ok(Request::Adjust(index.parse()?, steps(amount)?)),
        (None, _, _) => Err(anyhow!("adjust needs a step count")),
        _ => Err(anyhow!("adjust takes a throw and a step count")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn negative_indices_count_from_the_end() {
        assert_eq!(Index(-1).resolve(3).unwrap(), 3);
        assert_eq!(Index(-3).resolve(3).unwrap(), 1);
        assert_eq!(Index(1).resolve(3).unwrap(), 1);
        for out_of_range in [Index(-4), Index(4), Index(0)] {
            assert!(out_of_range.resolve(3).is_err(), "{out_of_range:?}");
        }
        assert!(Index::LAST.resolve(0).is_err());
    }

    #[test]
    fn every_request_survives_a_round_trip() {
        for request in [
            Request::Show(Target::All),
            Request::Hide(Target::Throws),
            Request::Toggle(Target::All),
            Request::Quit,
            Request::Reset,
            Request::Undo,
            Request::Redo,
            Request::Drop(Index(2)),
            Request::Adjust(Index(-1), -3),
            Request::Measure("/execute in x run tp @s 1 2 3 4 5".to_owned()),
            Request::Status,
        ] {
            let line = request.encode();
            assert_eq!(Request::parse(&line).unwrap(), request, "{line}");
        }
    }

    #[test]
    fn a_bare_request_means_the_obvious_thing() {
        assert_eq!(Request::parse("adjust +1").unwrap(), Request::Adjust(Index::LAST, 1));
        assert_eq!(Request::parse("adjust -2").unwrap(), Request::Adjust(Index::LAST, -2));
        assert_eq!(Request::parse("adjust 2 -1").unwrap(), Request::Adjust(Index(2), -1));
        assert_eq!(Request::parse("drop").unwrap(), Request::Drop(Index::LAST));
        assert_eq!(Request::parse("show").unwrap(), Request::Show(Target::All));
        assert_eq!(Request::parse("hide throws").unwrap(), Request::Hide(Target::Throws));
    }

    #[test]
    fn nonsense_is_refused() {
        for line in ["adjust", "adjust 1 2 3", "show sideways", "quit now", "measure", "fly"] {
            assert!(Request::parse(line).is_err(), "{line}");
        }
    }
}
