//! The socket the box takes its orders on.
//!
//! One connection is one request: a line, answered with a line. That line is
//! empty when there is nothing to say, starts with `error: ` when the request
//! was refused, and is otherwise whatever the request had to report.

use anyhow::{anyhow, Context, Result};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::action::Request;
use crate::control::Answer;

/// How long either end waits on the other. Both are local and send a single
/// line, so this only expires on something that has stopped talking. It
/// matters most when serving: that read happens on the event loop, and without
/// a bound anything that connected and then said nothing would stop the box.
const PATIENCE: Duration = Duration::from_millis(500);

pub fn default_socket() -> PathBuf {
    // `place_` rather than `get_`: this is the call that creates the runtime
    // directory, with the permissions the spec asks for.
    crate::config::directories()
        .place_runtime_file("socket")
        .unwrap_or_else(|_| PathBuf::from("/tmp/ninjabrain-box.sock"))
}

/// Claims `path`, clearing away a socket a crash left behind.
pub fn bind(path: &Path) -> Result<UnixListener> {
    if UnixStream::connect(path).is_ok() {
        return Err(anyhow!("another box is already listening on {}", path.display()));
    }
    let _ = std::fs::remove_file(path);
    let listener = UnixListener::bind(path)
        .with_context(|| format!("cannot listen on {}", path.display()))?;
    listener
        .set_nonblocking(true)
        .with_context(|| format!("{}", path.display()))?;
    Ok(listener)
}

/// Sends one request to a running box, and returns what it said.
pub fn request(path: &Path, line: &str) -> Result<String> {
    let mut stream = UnixStream::connect(path).context("no box")?;
    let _ = stream.set_read_timeout(Some(PATIENCE));
    let _ = stream.set_write_timeout(Some(PATIENCE));
    writeln!(stream, "{line}")
        .and_then(|()| stream.shutdown(std::net::Shutdown::Write))
        .context("cannot send the request")?;
    let mut answer = String::new();
    BufReader::new(&stream)
        .read_line(&mut answer)
        .context("cannot read the answer")?;
    let answer = answer.trim().to_owned();
    match answer.strip_prefix("error: ") {
        Some(message) => Err(anyhow!("{}", message)),
        None => Ok(answer),
    }
}

/// Serves whatever is waiting, handing each request to `act`.
pub fn accept(listener: &UnixListener, act: &mut dyn FnMut(&Request) -> Result<Answer>) {
    while let Ok((stream, _)) = listener.accept() {
        let _ = stream.set_read_timeout(Some(PATIENCE));
        let _ = stream.set_write_timeout(Some(PATIENCE));
        let mut line = String::new();
        if BufReader::new(&stream).read_line(&mut line).is_err() {
            continue;
        }
        let answer = match Request::parse(&line).and_then(|request| act(&request)) {
            Ok(Answer::Done) => String::new(),
            Ok(Answer::Text(text)) => text,
            Err(reason) => format!("error: {reason:#}"),
        };
        let mut stream = &stream;
        let _ = writeln!(stream, "{answer}");
    }
}
