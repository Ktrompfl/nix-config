//! Ninjabrain Bot in a box, with its panel on a Wayland overlay.
//!
//! The bot is an X11 program driven by hotkeys and a clipboard, and it draws
//! its window wherever the window manager puts it. Rather than fight any of
//! that, it is given a display of its own with nothing else on it: it runs
//! there exactly as published, reading a clipboard and hotkeys only the box
//! can reach, while its panel is drawn out here on a layer-shell surface that
//! can sit above a fullscreen game.
//!
//! - [`model`] is the box's own language: what the game measures, what the box
//!   believes the input should be, and what a calculator answers.
//! - [`backend`] owns a calculator and everything specific to it.
//! - [`control`] is the box: the session, its history, and the driving.
//! - [`frontend`] draws. It reads [`control`] rather than being part of it,
//!   which is what `--headless` leaves out.

mod action;
mod backend;
mod cli;
mod config;
mod control;
mod frontend;
mod ipc;
mod model;

use anyhow::{Context, Result, anyhow};
use calloop::{EventLoop, Interest, Mode, PostAction};
use calloop_wayland_source::WaylandSource;
use wayland_client::globals::registry_queue_init;
use wayland_client::Connection;

use backend::ninjabrain::Ninjabrain;
use cli::Invocation;
use config::Config;
use control::Control;
use frontend::{App, Text};

fn main() {
    if let Err(reason) = run() {
        eprintln!("ninjabrain-box: {reason:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    match cli::parse()? {
        Invocation::Run { headless: false } => windowed(),
        Invocation::Run { headless: true } => headless(),
        Invocation::Completions(shell) => {
            cli::completions(shell);
            Ok(())
        }
        Invocation::Send(request) => {
            let answer = ipc::request(&ipc::default_socket(), &request.encode())?;
            if !answer.is_empty() {
                println!("{answer}");
            }
            Ok(())
        }
    }
}

fn headless() -> Result<()> {
    let config = Config::load()?;
    let socket = ipc::default_socket();
    let listener = ipc::bind(&socket)?;

    let mut event_loop: EventLoop<'static, Control> =
        EventLoop::try_new().context("event loop")?;
    let handle = event_loop.handle();
    let (solutions, solution_source) = calloop::channel::channel::<model::Solution>();
    let mut control = Control::new(Box::new(Ninjabrain::start(&config, solutions)?));

    handle
        .insert_source(
            calloop::generic::Generic::new(listener, Interest::READ, Mode::Level),
            |_, listener, control: &mut Control| {
                ipc::accept(listener, &mut |request| control.act(request));
                Ok(PostAction::Continue)
            },
        )
        .map_err(|error| anyhow!("cannot watch {}: {error}", socket.display()))?;
    handle
        .insert_source(solution_source, |event, _, control: &mut Control| {
            if let calloop::channel::Event::Msg(solution) = event {
                control.absorb(solution);
            }
        })
        .map_err(|error| anyhow!("solution channel: {error}"))?;

    loop {
        event_loop
            .dispatch(std::time::Duration::from_millis(100), &mut control)
            .context("event loop")?;
        control.tick();
        if control.exit {
            let _ = std::fs::remove_file(&socket);
            return Ok(());
        }
    }
}

fn windowed() -> Result<()> {
    let config = Config::load()?;
    let text = Text::load(&config)?;
    let socket = ipc::default_socket();
    let listener = ipc::bind(&socket)?;

    let connection = Connection::connect_to_env()
        .context("cannot reach the compositor")?;
    let (globals, queue) = registry_queue_init::<App>(&connection)
        .context("cannot set up the compositor connection")?;
    let handle = queue.handle();

    let mut event_loop: EventLoop<'static, App> =
        EventLoop::try_new().context("event loop")?;
    let loop_handle = event_loop.handle();

    let (clipboard_sink, clipboard_source) = calloop::channel::channel::<String>();
    let (solutions, solution_source) = calloop::channel::channel::<model::Solution>();
    let control = Control::new(Box::new(Ninjabrain::start(&config, solutions)?));
    let mut app = App::new(&globals, &handle, config, text, control, clipboard_sink)?;

    WaylandSource::new(connection, queue)
        .insert(loop_handle.clone())
        .map_err(|error| anyhow!("cannot watch the compositor: {error}"))?;
    loop_handle
        .insert_source(
            calloop::generic::Generic::new(listener, Interest::READ, Mode::Level),
            |_, listener, app: &mut App| {
                ipc::accept(listener, &mut |request| app.act(request));
                Ok(PostAction::Continue)
            },
        )
        .map_err(|error| anyhow!("cannot watch {}: {error}", socket.display()))?;
    loop_handle
        .insert_source(clipboard_source, |event, _, app: &mut App| {
            if let calloop::channel::Event::Msg(text) = event {
                app.on_clipboard(&text);
            }
        })
        .map_err(|error| anyhow!("clipboard channel: {error}"))?;
    loop_handle
        .insert_source(solution_source, |event, _, app: &mut App| {
            if let calloop::channel::Event::Msg(solution) = event {
                app.update(solution);
            }
        })
        .map_err(|error| anyhow!("solution channel: {error}"))?;

    app.refresh_visibility();
    loop {
        event_loop
            .dispatch(std::time::Duration::from_millis(100), &mut app)
            .context("event loop")?;
        app.tick();
        if app.control.exit {
            // Only reached on a clean stop; a signal leaves this behind, and
            // the next start clears it away.
            let _ = std::fs::remove_file(&socket);
            return Ok(());
        }
    }
}
