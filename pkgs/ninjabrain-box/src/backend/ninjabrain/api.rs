//! Ninjabrain Bot's HTTP API: its wire format, and the translation out of it.
//!
//! Everything in this file is the bot's spelling -- `xInOverworld`,
//! `resultType`, Swing HTML in the messages -- and none of it escapes. What
//! leaves is [`Solution`], which says the same things in the box's words.
//!
//! The bot answers each query on its own stream, so they are followed and
//! merged here rather than anywhere that would have to know there were three.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Deserialize;

use crate::model::{
    Blind, Improvement, Message, Player, Prediction, Quality, Severity, Solution, Status,
    ThrowReport,
};

// --- what the bot sends -------------------------------------------------

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum ResultType {
    #[default]
    None,
    Triangulation,
    Failed,
    Blind,
    /// Divine and all-advancements, which the box cannot enter and does not
    /// draw. Kept only so an unexpected answer parses rather than being lost.
    #[serde(other)]
    Other,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WirePrediction {
    certainty: f64,
    chunk_x: i32,
    chunk_z: i32,
    overworld_distance: f64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WireThrow {
    #[serde(rename = "xInOverworld")]
    x: f64,
    #[serde(rename = "zInOverworld")]
    z: f64,
    angle: f64,
    #[serde(default)]
    angle_without_correction: f64,
    #[serde(default)]
    correction: f64,
    #[serde(default)]
    correction_increments: i32,
    #[serde(default)]
    error: f64,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WirePlayer {
    #[serde(rename = "xInOverworld")]
    x: Option<f64>,
    #[serde(rename = "zInOverworld")]
    z: Option<f64>,
    horizontal_angle: Option<f64>,
    #[serde(default)]
    is_in_nether: bool,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Stronghold {
    result_type: ResultType,
    predictions: Vec<WirePrediction>,
    eye_throws: Vec<WireThrow>,
    player_position: WirePlayer,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum BlindEvaluation {
    Excellent,
    HighrollGood,
    HighrollOkay,
    BadButInRing,
    Bad,
    NotInRing,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct WireBlindResult {
    #[serde(rename = "xInNether")]
    x: f64,
    #[serde(rename = "zInNether")]
    z: f64,
    highroll_probability: f64,
    highroll_threshold: f64,
    improve_direction: f64,
    improve_distance: f64,
    evaluation: Option<BlindEvaluation>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct BlindWire {
    blind_result: WireBlindResult,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum WireSeverity {
    Info,
    #[default]
    Warning,
    Error,
}

#[derive(Clone, Debug, Deserialize)]
struct WireMessage {
    #[serde(default)]
    severity: WireSeverity,
    #[serde(default)]
    message: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MessagesWire {
    information_messages: Vec<WireMessage>,
}

// --- what the box sees --------------------------------------------------

/// The four answers, merged. Kept in the bot's own terms until asked for a
/// [`Solution`], because each stream only ever refreshes its own quarter.
#[derive(Default)]
pub struct Answers {
    stronghold: Stronghold,
    blind: BlindWire,
    messages: MessagesWire,
}

impl Answers {
    /// What the bot's throw list says the session must be, as far as it can be
    /// read off the answer. Measurements are not in the API, so they are left
    /// for the journal to fill in.
    pub fn observed_throws(&self) -> Vec<ObservedThrow> {
        self.stronghold
            .eye_throws
            .iter()
            .map(|throw| ObservedThrow {
                x: throw.x,
                z: throw.z,
                angle_without_correction: throw.angle_without_correction,
                corrections: throw.correction_increments,
            })
            .collect()
    }

    pub fn solution(&self) -> Solution {
        let stronghold = &self.stronghold;
        let status = match stronghold.result_type {
            ResultType::Failed => Status::Failed,
            ResultType::Triangulation if !stronghold.predictions.is_empty() => Status::Solved,
            _ => Status::Waiting,
        };

        Solution {
            status,
            predictions: stronghold
                .predictions
                .iter()
                .map(|p| Prediction {
                    certainty: p.certainty,
                    chunk: (p.chunk_x, p.chunk_z),
                    overworld_distance: p.overworld_distance,
                })
                .collect(),
            throws: stronghold
                .eye_throws
                .iter()
                .map(|t| ThrowReport {
                    x: t.x,
                    z: t.z,
                    angle: t.angle,
                    correction: t.correction,
                    correction_steps: t.correction_increments,
                    error: t.error,
                })
                .collect(),
            player: Player {
                position: stronghold
                    .player_position
                    .x
                    .zip(stronghold.player_position.z),
                horizontal_angle: stronghold.player_position.horizontal_angle,
                in_nether: stronghold.player_position.is_in_nether,
            },
            blind: self.blind_result(),
            messages: self
                .messages
                .information_messages
                .iter()
                .map(|m| Message {
                    severity: match m.severity {
                        WireSeverity::Info => Severity::Info,
                        WireSeverity::Warning => Severity::Warning,
                        WireSeverity::Error => Severity::Error,
                    },
                    text: plain(&m.message),
                })
                .collect(),
        }
    }

    fn blind_result(&self) -> Option<Blind> {
        let result = &self.blind.blind_result;
        let quality = match result.evaluation? {
            BlindEvaluation::Excellent => Quality::Excellent,
            BlindEvaluation::HighrollGood => Quality::Good,
            BlindEvaluation::HighrollOkay => Quality::Okay,
            BlindEvaluation::BadButInRing => Quality::Poor,
            BlindEvaluation::Bad => Quality::Bad,
            BlindEvaluation::NotInRing => Quality::OutOfRange,
        };
        Some(Blind {
            nether: (result.x, result.z),
            quality,
            highroll_probability: result.highroll_probability,
            highroll_threshold: result.highroll_threshold,
            // Only worth saying while there is somewhere better to stand.
            improve: (result.improve_distance >= 1.0).then_some(Improvement {
                distance: result.improve_distance,
                direction: result.improve_direction,
            }),
        })
    }

}

/// One throw as the bot reports it, before the journal has matched it to the
/// measurement it was made from.
pub struct ObservedThrow {
    pub x: f64,
    pub z: f64,
    pub angle_without_correction: f64,
    pub corrections: i32,
}

/// The bot writes its messages as Swing HTML. This is not a parser and does
/// not need to be: the messages are prose with the odd `<b>` in them, and the
/// box is owed text.
fn plain(message: &str) -> String {
    let mut out = String::with_capacity(message.len());
    let mut characters = message.chars().peekable();
    while let Some(character) = characters.next() {
        match character {
            // A tag can stand between two words that would otherwise run
            // together, so it becomes a space -- but only when something
            // word-like follows, or "<b>4</b>." would come out as "4 .".
            '<' => {
                for character in characters.by_ref() {
                    if character == '>' {
                        break;
                    }
                }
                if characters.peek().is_some_and(|next| next.is_alphanumeric()) {
                    out.push(' ');
                }
            }
            '&' => {
                let mut entity = String::new();
                while let Some(&character) = characters.peek() {
                    characters.next();
                    if character == ';' || entity.len() >= 8 {
                        break;
                    }
                    entity.push(character);
                }
                out.push_str(match entity.as_str() {
                    "amp" => "&",
                    "lt" => "<",
                    "gt" => ">",
                    "quot" => "\"",
                    _ => " ",
                });
            }
            _ => out.push(character),
        }
    }
    out.split_whitespace().collect::<Vec<&str>>().join(" ")
}

// --- following the streams ----------------------------------------------

/// Follows all four queries, merging them, and hands a whole [`Solution`] to
/// `sink` whenever any of them changes.
pub fn watch(
    address: &str,
    answers: Arc<Mutex<Answers>>,
    sink: calloop::channel::Sender<Solution>,
) {
    let apply = |answers: &mut Answers, update: Update| match update {
        Update::Stronghold(v) => answers.stronghold = v,
        Update::Blind(v) => answers.blind = v,
        Update::Messages(v) => answers.messages = v,
    };
    for (endpoint, parse) in ENDPOINTS {
        let watcher = Watcher {
            address: address.to_owned(),
            endpoint,
            parse,
        };
        let answers = answers.clone();
        let sink = sink.clone();
        std::thread::Builder::new()
            .name(format!("ninjabrain-box-{endpoint}"))
            .spawn(move || {
                watcher.run(&move |update| {
                    let solution = {
                        let mut answers = answers.lock().expect("the answers are not poisoned");
                        apply(&mut answers, update);
                        answers.solution()
                    };
                    sink.send(solution).is_ok()
                })
            })
            .expect("a thread per stream");
    }
}

enum Update {
    Stronghold(Stronghold),
    Blind(BlindWire),
    Messages(MessagesWire),
}

type Parse = fn(&str) -> Option<Update>;

const ENDPOINTS: [(&str, Parse); 3] = [
    ("stronghold", |body| {
        serde_json::from_str(body).ok().map(Update::Stronghold)
    }),
    ("blind", |body| {
        serde_json::from_str(body).ok().map(Update::Blind)
    }),
    ("information-messages", |body| {
        serde_json::from_str(body).ok().map(Update::Messages)
    }),
];

/// Reads one of the bot's queries and keeps reading it.
///
/// The API offers a server-sent event stream per query, which is a plain
/// `text/event-stream` over HTTP/1.1 -- one blank-line-separated record per
/// change, each a single `data:` line. That is little enough protocol to speak
/// directly, and saves the overlay an HTTP client.
struct Watcher {
    address: String,
    endpoint: &'static str,
    parse: Parse,
}

impl Watcher {
    /// Runs until `deliver` says nobody is listening any more.
    fn run(self, deliver: &dyn Fn(Update) -> bool) {
        loop {
            if !self.stream(self.parse, deliver) {
                return;
            }
            std::thread::sleep(Duration::from_secs(2));
        }
    }

    /// One connection's worth of events. `false` means stop for good.
    fn stream(&self, parse: Parse, deliver: &dyn Fn(Update) -> bool) -> bool {
        let Ok(stream) = TcpStream::connect(&self.address) else {
            return true;
        };
        let _ = stream.set_read_timeout(None);
        let mut writer = &stream;
        if writer
            .write_all(
                format!(
                    "GET /api/v1/{}/events HTTP/1.1\r\n\
                     Host: localhost\r\n\
                     Accept: text/event-stream\r\n\
                     Connection: keep-alive\r\n\r\n",
                    self.endpoint
                )
                .as_bytes(),
            )
            .is_err()
        {
            return true;
        }

        let mut reader = BufReader::new(&stream);
        let mut line = String::new();
        // Headers, up to the blank line that ends them.
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) | Err(_) => return true,
                Ok(_) if line.trim().is_empty() => break,
                Ok(_) => {}
            }
        }

        // The body is chunked, but every chunk is one `data: {...}` record, so
        // picking the data lines out of the stream is enough and the chunk
        // sizes can be ignored as the noise they are.
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) | Err(_) => return true,
                Ok(_) => {}
            }
            let Some(payload) = line.trim_start().strip_prefix("data: ") else {
                continue;
            };
            let Some(update) = parse(payload.trim()) else {
                continue;
            };
            if !deliver(update) {
                return false;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_the_bots_markup() {
        assert_eq!(
            plain("<html>Go left <b>12</b> blocks, or right <b>4</b>."),
            "Go left 12 blocks, or right 4."
        );
        assert_eq!(plain("a &amp; b &lt;c&gt;"), "a & b <c>");
        // A tag between two words is still a boundary.
        assert_eq!(plain("left<br>right"), "left right");
        assert_eq!(plain("<html>Detected <b>unusually</b> large errors."), "Detected unusually large errors.");
    }

    #[test]
    fn blind_mode_is_not_a_missing_answer() {
        let mut answers = Answers::default();
        answers.stronghold.result_type = ResultType::Blind;
        answers.blind.blind_result.evaluation = Some(BlindEvaluation::Excellent);
        let solution = answers.solution();
        assert!(solution.blind.is_some());
        assert_eq!(solution.placeholder(), None);
    }
}
