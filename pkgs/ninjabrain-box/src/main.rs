//! Ninjabrain Bot in a box, with its panel on a Wayland overlay.
//!
//! The bot is an X11 program that takes its input from an X server, is
//! configured through a window, and draws that window wherever the window
//! manager puts it. Rather than fight any of that, this gives it a display of
//! its own with nothing else on it: the bot runs there exactly as published,
//! reading a clipboard and hotkeys that only the overlay can reach, while its
//! panel is drawn out here on a layer-shell surface that can sit above a
//! fullscreen game.

mod action;
mod app;
mod bot;
mod clipboard;
mod config;
mod ipc;
mod model;
mod prefs;
mod render;
mod text;
mod toplevel;
mod xserver;

use std::path::PathBuf;
use std::process::Command;

use calloop::{EventLoop, Interest, Mode, PostAction};
use calloop_wayland_source::WaylandSource;
use wayland_client::globals::registry_queue_init;
use wayland_client::Connection;

use action::Action;
use app::App;
use config::Config;
use text::Text;
use xserver::XServer;

fn main() {
    if let Err(reason) = run() {
        eprintln!("ninjabrain-box: {reason}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    match arguments.first().map(String::as_str) {
        Some("--help" | "-h") => {
            print_help();
            Ok(())
        }
        Some("--list") => {
            for name in Action::names() {
                println!("{name}");
            }
            Ok(())
        }
        None => daemon(),
        Some(_) => forward(&arguments),
    }
}

fn print_help() {
    println!(
        "Usage: ninjabrain-box [ACTION...]\n\
         \n\
         With no arguments, starts Ninjabrain Bot and its overlay.\n\
         With actions, sends them to the running overlay.\n\
         \n\
         Actions:\n  {}\n\
         \n\
         Options:\n  --list   list the actions\n  --help   this",
        Action::names().join("\n  ")
    );
}

/// Sends actions on, preferring the overlay because it understands all of them.
fn forward(arguments: &[String]) -> Result<(), String> {
    // Checked here only so that a typo is answered without a round trip; the
    // overlay parses them again, and is the only thing that can run them --
    // it holds the box the bot is in.
    for name in arguments {
        if Action::parse(name).is_none() {
            return Err(format!("no such action: {name}"));
        }
    }
    ipc::request(&ipc::default_socket(), &format!("{}\n", arguments.join(" "))).map(drop)
}

/// Where this overlay's state lives, which is also the home the bot's JVM is
/// pointed at for its preferences.
fn state_directory() -> PathBuf {
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))
        .unwrap_or_else(|| PathBuf::from("/tmp"));
    base.join("ninjabrain-box")
}

/// The font the table is set in.
pub fn font_bytes(config: &Config) -> Result<Vec<u8>, String> {
    let path = config
        .window
        .font
        .clone()
        .or_else(|| std::env::var_os("NINJABRAIN_BOX_FONT").map(PathBuf::from))
        .ok_or("no font configured, and NINJABRAIN_BOX_FONT is unset")?;
    std::fs::read(&path).map_err(|error| format!("cannot read {}: {error}", path.display()))
}

fn daemon() -> Result<(), String> {
    let config = Config::load()?;
    let text = Text::new(&font_bytes(&config)?, config.window.font_size)?;
    let socket_path = ipc::default_socket();
    let listener = ipc::bind(&socket_path)?;

    // Declared in this order so that the bot is stopped before its display is.
    let x = std::rc::Rc::new(XServer::start()?);
    let _bot = start_bot(&config, &x)?;

    let connection = Connection::connect_to_env()
        .map_err(|error| format!("cannot reach the compositor: {error}"))?;
    let (globals, queue) = registry_queue_init::<App>(&connection)
        .map_err(|error| format!("cannot set up the compositor connection: {error}"))?;
    let handle = queue.handle();

    let mut event_loop: EventLoop<'static, App> =
        EventLoop::try_new().map_err(|error| format!("event loop: {error}"))?;
    let loop_handle = event_loop.handle();

    let (clipboard_sink, clipboard_source) = calloop::channel::channel::<String>();
    let (state_sink, state_source) = calloop::channel::channel::<model::Stronghold>();

    let mut app = App::new(
        &globals,
        &handle,
        config.clone(),
        text,
        x.clone(),
        clipboard_sink,
    )?;

    WaylandSource::new(connection, queue)
        .insert(loop_handle.clone())
        .map_err(|error| format!("cannot watch the compositor: {error}"))?;

    loop_handle
        .insert_source(
            calloop::generic::Generic::new(listener, Interest::READ, Mode::Level),
            |_, listener, app: &mut App| {
                ipc::accept(listener, app);
                Ok(PostAction::Continue)
            },
        )
        .map_err(|error| format!("cannot watch {}: {error}", socket_path.display()))?;

    let measurements = x.clone();
    loop_handle
        .insert_source(clipboard_source, move |event, _, _: &mut App| {
            let calloop::channel::Event::Msg(text) = event else {
                return;
            };
            if clipboard::is_measurement(&text) {
                measurements.set_clipboard(text.trim());
            }
        })
        .map_err(|error| format!("clipboard channel: {error}"))?;

    loop_handle
        .insert_source(state_source, |event, _, app: &mut App| {
            if let calloop::channel::Event::Msg(state) = event {
                app.update(state);
            }
        })
        .map_err(|error| format!("state channel: {error}"))?;

    // The bot sends a subscriber the current state as soon as it subscribes,
    // so following the stream is also how the panel is first filled in.
    let address = config.bot.api.clone();
    std::thread::spawn(move || bot::Watcher::new(&address).run(state_sink));

    app.refresh_visibility();
    loop {
        event_loop
            .dispatch(std::time::Duration::from_millis(250), &mut app)
            .map_err(|error| format!("event loop: {error}"))?;
        if app.exit {
            // Only reached on a clean stop; a signal leaves this behind, and
            // the next start clears it away.
            let _ = std::fs::remove_file(&socket_path);
            return Ok(());
        }
    }
}

/// A bot process that does not outlive the overlay.
///
/// Two belts: this kills it on the way out of a clean shutdown, and the child
/// itself asks the kernel to be signalled if the overlay dies without getting
/// that far -- which it will, since the overlay is meant to be stopped with a
/// signal and Rust runs no destructors for those. The same goes for the box.
struct Bot(std::process::Child);

impl Drop for Bot {
    fn drop(&mut self) {
        xserver::stop(&mut self.0);
    }
}

/// Starts a bot of the overlay's own, inside the box, with preferences to
/// match.
///
/// Nothing is done to the bot itself. It is the published release, started the
/// way its own launcher starts it, pointed at a display and a preferences file
/// that happen to be ours.
fn start_bot(config: &Config, x: &XServer) -> Result<Bot, String> {
    let bot = std::env::var_os("NINJABRAIN_BOX_BOT")
        .ok_or("NINJABRAIN_BOX_BOT is unset; this build is incomplete")?;

    // The bot's API port is fixed, and a second bot that cannot have it just
    // logs and carries on -- leaving the overlay reading the first one's
    // numbers and showing them as its own. Better to say so and stop.
    if std::net::TcpStream::connect(&config.bot.api).is_ok() {
        return Err(format!(
            "something is already serving the Ninjabrain API on {}, \
             most likely another ninjabrain-box; stop that one first",
            config.bot.api
        ));
    }

    let state = state_directory();
    prefs::write(&state, &config.bot.settings)?;

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
        .map(Bot)
        .map_err(|error| format!("cannot start {}: {error}", PathBuf::from(bot).display()))
}
