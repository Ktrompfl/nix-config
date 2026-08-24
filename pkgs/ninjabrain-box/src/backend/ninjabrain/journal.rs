//! The measurements this backend has handed the bot.
//!
//! A measurement is never reconstructed from an answer: the angle the API
//! reports has the crosshair correction and a snap already folded in, and the
//! snap has changed between releases and does not invert. The original line is
//! kept instead, on disk as well as in memory, because the bot restores its
//! own throws on startup and a throw whose measurement is unknown can never be
//! entered again.

use std::io::Write as _;
use std::path::PathBuf;

use crate::model::Measurement;

use super::api::ObservedThrow;

/// Enough for any session; a stronghold is found in a dozen throws or never.
const LIMIT: usize = 64;

/// Coordinates go through the bot untouched, so a throw's are its
/// measurement's exactly, give or take the JSON.
const SAME_PLACE: f64 = 1e-6;

/// What counts as the same angle when telling two throws made from the same
/// spot apart.
///
/// Wide enough to swallow the bot's snapping whatever the sensitivity, and far
/// narrower than two throws a person aimed separately: to be caught out by
/// this you would have to stand on one block, throw twice without moving a
/// pixel, and have the two angles land within a fifth of a degree.
const SAME_ANGLE: f64 = 0.2;

pub struct Journal {
    path: Option<PathBuf>,
    lines: Vec<String>,
}

impl Journal {
    pub fn load(directory: &std::path::Path) -> Journal {
        let path = directory.join("measurements");
        let lines = std::fs::read_to_string(&path)
            .map(|text| text.lines().map(str::to_owned).collect())
            .unwrap_or_default();
        Journal {
            path: Some(path),
            lines,
        }
    }

    pub fn record(&mut self, measurement: &Measurement) {
        if self.lines.last().is_some_and(|last| *last == measurement.line) {
            return;
        }
        self.lines.push(measurement.line.clone());
        let excess = self.lines.len().saturating_sub(LIMIT);
        self.lines.drain(..excess);
        self.save();
    }

    /// Best effort: losing the journal costs the ability to rebuild a list,
    /// not the ability to measure, so a failure is not worth stopping for --
    /// but it is worth saying once.
    fn save(&mut self) {
        let Some(path) = self.path.as_ref() else {
            return;
        };
        let written = std::fs::create_dir_all(path.parent().expect("the journal has a parent"))
            .and_then(|()| std::fs::File::create(path))
            .and_then(|mut file| {
                for line in &self.lines {
                    writeln!(file, "{line}")?;
                }
                Ok(())
            });
        if let Err(error) = written {
            eprintln!(
                "ninjabrain-box: cannot keep {}: {error}; throws cannot be rebuilt",
                path.display()
            );
            self.path = None;
        }
    }

    /// The measurement `throw` was made from, if it is still known. The
    /// nearest angle wins, so two throws from one spot go back to their own
    /// lines.
    pub fn measurement_for(
        &self,
        throw: &ObservedThrow,
        crosshair_correction: f64,
    ) -> Option<Measurement> {
        self.lines
            .iter()
            .filter_map(|line| {
                let measurement = Measurement::parse(line)?;
                let elsewhere = (measurement.x - throw.x).abs() >= SAME_PLACE
                    || (measurement.z - throw.z).abs() >= SAME_PLACE;
                if elsewhere {
                    return None;
                }
                // The bot adds the crosshair correction, which is known, and
                // then snaps -- which is what the tolerance is for.
                let angle = crate::model::normalize_angle(
                    measurement.yaw + crosshair_correction - throw.angle_without_correction,
                )
                .abs();
                (angle < SAME_ANGLE).then_some((measurement, angle))
            })
            .min_by(|(_, a), (_, b)| a.total_cmp(b))
            .map(|(measurement, _)| measurement)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn journal(lines: &[&str]) -> Journal {
        Journal {
            path: None,
            lines: lines.iter().map(|l| (*l).to_owned()).collect(),
        }
    }

    fn line(x: f64, yaw: f64) -> String {
        format!("/execute in minecraft:overworld run tp @s {x:.2} 68.00 -20.25 {yaw:.2} -31.00")
    }

    fn observed(x: f64, angle: f64) -> ObservedThrow {
        ObservedThrow {
            x,
            z: -20.25,
            angle_without_correction: angle,
            corrections: 0,
        }
    }

    /// The bot snaps the angle it reports by a fraction of a degree; a throw
    /// still has to be recognised as its measurement despite that.
    #[test]
    fn matches_through_the_bots_snapping() {
        let journal = journal(&[&line(10.0, -45.60), &line(10.0, -12.30)]);
        let found = journal
            .measurement_for(&observed(10.0, -45.606), 0.0)
            .expect("matched");
        assert_eq!(found.yaw, -45.60);

        // A second throw from the same spot goes back to its own line.
        let found = journal
            .measurement_for(&observed(10.0, -12.294), 0.0)
            .expect("matched");
        assert_eq!(found.yaw, -12.30);
    }

    #[test]
    fn does_not_match_a_different_throw() {
        let journal = journal(&[&line(10.0, -45.60)]);
        assert!(journal.measurement_for(&observed(11.0, -45.60), 0.0).is_none());
        assert!(journal.measurement_for(&observed(10.0, -44.00), 0.0).is_none());
    }

    /// The crosshair correction is added by the bot before it reports, so a
    /// configured correction has to be allowed for when matching back.
    #[test]
    fn allows_for_the_crosshair_correction() {
        let journal = journal(&[&line(10.0, -45.60)]);
        assert!(journal.measurement_for(&observed(10.0, -45.10), 0.5).is_some());
        assert!(journal.measurement_for(&observed(10.0, -45.10), 0.0).is_none());
    }
}
