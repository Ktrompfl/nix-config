//! Reading the bot.
//!
//! Everything going the other way -- measurements and hotkeys -- goes in
//! through the box the bot runs in, as clipboard and key events. What comes
//! back comes over the HTTP API the bot already has.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::time::Duration;

use crate::model::Stronghold;

/// Reads the bot's stronghold state and keeps reading it.
///
/// The API offers a server-sent event stream per query, which is a plain
/// `text/event-stream` over HTTP/1.1 -- one blank-line-separated record per
/// change, each a single `data:` line. That is little enough protocol to speak
/// directly, and saves the overlay an HTTP client.
pub struct Watcher {
    address: String,
}

impl Watcher {
    pub fn new(address: &str) -> Watcher {
        Watcher {
            address: address.to_owned(),
        }
    }

    /// Runs until `sink` is gone, reconnecting whenever the bot is not there.
    pub fn run(self, sink: calloop::channel::Sender<Stronghold>) {
        loop {
            match self.stream(&sink) {
                Ok(()) => {}
                Err(_disconnected) => return,
            }
            std::thread::sleep(Duration::from_secs(2));
        }
    }

    /// One connection's worth of events. `Err` means the receiver is gone.
    fn stream(&self, sink: &calloop::channel::Sender<Stronghold>) -> Result<(), ()> {
        let Ok(stream) = TcpStream::connect(&self.address) else {
            return Ok(());
        };
        let _ = stream.set_read_timeout(None);
        let mut writer = &stream;
        if writer
            .write_all(
                b"GET /api/v1/stronghold/events HTTP/1.1\r\n\
                  Host: localhost\r\n\
                  Accept: text/event-stream\r\n\
                  Connection: keep-alive\r\n\r\n",
            )
            .is_err()
        {
            return Ok(());
        }

        let mut reader = BufReader::new(&stream);
        let mut line = String::new();
        // Headers, up to the blank line that ends them.
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => return Ok(()),
                Ok(_) if line.trim().is_empty() => break,
                Ok(_) => {}
                Err(_) => return Ok(()),
            }
        }

        // The body is chunked, but every chunk is one `data: {...}` record, so
        // picking the data lines out of the stream is enough and the chunk
        // sizes can be ignored as the noise they are.
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => return Ok(()),
                Ok(_) => {}
                Err(_) => return Ok(()),
            }
            let Some(payload) = line.trim_start().strip_prefix("data: ") else {
                continue;
            };
            let Ok(state) = serde_json::from_str::<Stronghold>(payload.trim()) else {
                continue;
            };
            sink.send(state).map_err(drop)?;
        }
    }
}

