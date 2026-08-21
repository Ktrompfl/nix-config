//! The socket the overlay takes its orders on.
//!
//! One connection is one request: a line of action names, answered with an
//! empty line, or one starting `error: `. The overlay is the only thing that
//! can run an action -- it holds the box the bot is in -- so this is also
//! what the command line talks to.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// How long either end waits on the other.
///
/// Both are local and send a single line, so this only ever expires on
/// something that has stopped talking. It matters most on the serving side:
/// that read happens on the event loop, and without a bound anything that
/// opened the socket and then said nothing would stop the overlay dead.
const PATIENCE: Duration = Duration::from_millis(500);

use crate::action::Action;
use crate::app::App;

pub fn default_socket() -> PathBuf {
    let runtime = std::env::var_os("XDG_RUNTIME_DIR").unwrap_or_else(|| "/tmp".into());
    PathBuf::from(runtime).join("ninjabrain-box.sock")
}

/// Claims `path`, clearing away a socket a crash left behind.
pub fn bind(path: &Path) -> Result<UnixListener, String> {
    if answers(path) {
        return Err(format!("another overlay is already listening on {}", path.display()));
    }
    let _ = std::fs::remove_file(path);
    let listener = UnixListener::bind(path)
        .map_err(|error| format!("cannot listen on {}: {error}", path.display()))?;
    listener
        .set_nonblocking(true)
        .map_err(|error| format!("{}: {error}", path.display()))?;
    Ok(listener)
}

fn answers(path: &Path) -> bool {
    UnixStream::connect(path).is_ok()
}

/// Sends one request to a running overlay.
pub fn request(path: &Path, line: &str) -> Result<String, String> {
    let mut stream =
        UnixStream::connect(path).map_err(|error| format!("no overlay: {error}"))?;
    let _ = stream.set_read_timeout(Some(PATIENCE));
    let _ = stream.set_write_timeout(Some(PATIENCE));
    stream
        .write_all(line.as_bytes())
        .and_then(|()| stream.shutdown(std::net::Shutdown::Write))
        .map_err(|error| error.to_string())?;
    let mut answer = String::new();
    BufReader::new(&stream)
        .read_line(&mut answer)
        .map_err(|error| error.to_string())?;
    let answer = answer.trim().to_owned();
    match answer.strip_prefix("error: ") {
        Some(message) => Err(message.to_owned()),
        None => Ok(answer),
    }
}

/// Serves whatever is waiting on the listener.
pub fn accept(listener: &UnixListener, app: &mut App) {
    while let Ok((stream, _)) = listener.accept() {
        let _ = stream.set_read_timeout(Some(PATIENCE));
        let _ = stream.set_write_timeout(Some(PATIENCE));
        let mut line = String::new();
        if BufReader::new(&stream).read_line(&mut line).is_err() {
            continue;
        }
        let answer = handle(line.trim(), app);
        let mut stream = &stream;
        let _ = writeln!(stream, "{answer}");
    }
}

fn handle(line: &str, app: &mut App) -> String {
    if line.is_empty() {
        return String::new();
    }
    // Parse the lot before running any of it, so a typo runs nothing.
    let mut actions = Vec::new();
    for name in line.split_whitespace() {
        match Action::parse(name) {
            Some(action) => actions.push(action),
            None => return format!("error: no such action: {name}"),
        }
    }
    for action in actions {
        app.act(action);
    }
    String::new()
}
