//! The box the bot runs in.
//!
//! Ninjabrain Bot is a Swing program, and Swing needs an X server -- asking
//! for a headless JVM gets a `HeadlessException` out of the splash screen
//! before `main` has finished. So it gets one of its own: a display with no
//! output, on which nothing is ever seen and nothing else is ever a client.
//!
//! That is also how it is driven. The bot watches the X clipboard and grabs
//! its hotkeys with XRecord, and inside this box both are ours alone -- the
//! overlay owns the clipboard selection and replays keys through XTEST. Every
//! byte the bot executes is the release as published, taking its input the way
//! it was written to.

use std::path::Path;
use std::process::{Child, Command};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

use x11rb::connection::Connection;
use x11rb::protocol::xproto::{
    self, AtomEnum, ChangeWindowAttributesAux, ConnectionExt as _, CreateWindowAux, EventMask,
    PropMode, SelectionNotifyEvent, WindowClass,
};
use x11rb::protocol::xtest::ConnectionExt as _;
use x11rb::protocol::Event;
use x11rb::rust_connection::RustConnection;
use x11rb::wrapper::ConnectionExt as _;
use x11rb::CURRENT_TIME;

/// Displays to look at for a free one. Well clear of anything a session or a
/// compositor's Xwayland would have taken.
const DISPLAYS: std::ops::Range<u32> = 90..120;

/// Big enough for the bot's largest window, small enough to be nothing. The
/// framebuffer is the only thing this costs, and nothing ever draws to it.
const SCREEN: &str = "1280x1024x24";

enum Request {
    /// Put text on the box's clipboard, for the bot to find.
    Clipboard(String),
    /// Press and release a key, for the bot's hotkey listener to see.
    Key(u32),
}

pub struct XServer {
    display: String,
    server: Child,
    requests: Sender<Request>,
}

impl XServer {
    /// Starts a display and takes ownership of its clipboard.
    pub fn start() -> Result<XServer, String> {
        let (display, server) = spawn()?;

        // Connect once here so that a failure is reported now rather than
        // swallowed by the thread.
        let (connection, screen) = RustConnection::connect(Some(&display))
            .map_err(|error| format!("cannot reach the box on {display}: {error}"))?;

        let (sender, receiver) = mpsc::channel();
        let owned = display.clone();
        std::thread::Builder::new()
            .name("ninjabrain-box-x11".into())
            .spawn(move || {
                if let Err(reason) = serve(connection, screen, receiver) {
                    eprintln!("ninjabrain-box: {owned}: {reason}");
                }
            })
            .map_err(|error| format!("cannot start the X thread: {error}"))?;

        Ok(XServer {
            display,
            server,
            requests: sender,
        })
    }

    pub fn display(&self) -> &str {
        &self.display
    }

    pub fn set_clipboard(&self, text: &str) {
        let _ = self.requests.send(Request::Clipboard(text.to_owned()));
    }

    /// Presses the key carrying `keysym`, whatever keycode it sits on.
    pub fn press(&self, keysym: u32) {
        let _ = self.requests.send(Request::Key(keysym));
    }
}

impl Drop for XServer {
    fn drop(&mut self) {
        stop(&mut self.server);
    }
}

/// Stops a child, giving it the chance to tidy up first.
///
/// `Child::kill` is SIGKILL, which neither the bot nor the X server survives
/// gracefully: the one skips its shutdown hooks and never writes its save
/// state, and the other leaves its socket behind in `/tmp/.X11-unix`. So ask
/// politely, and only insist if that does not work.
pub fn stop(child: &mut Child) {
    if let Some(pid) = rustix::process::Pid::from_raw(child.id() as i32) {
        let _ = rustix::process::kill_process(pid, rustix::process::Signal::Term);
    }
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if matches!(child.try_wait(), Ok(Some(_))) {
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let _ = child.kill();
    let _ = child.wait();
}

/// Starts an X server on the first display nothing else has claimed.
fn spawn() -> Result<(String, Child), String> {
    let mut failures = Vec::new();
    for number in DISPLAYS {
        let socket = format!("/tmp/.X11-unix/X{number}");
        match std::os::unix::net::UnixStream::connect(&socket) {
            // Somebody is home.
            Ok(_) => continue,
            // A socket nothing answers on is one a server was killed out
            // from under. Left alone it would cost a display number every
            // time, so take it back.
            Err(_) if Path::new(&socket).exists() => {
                if std::fs::remove_file(&socket).is_err() {
                    continue;
                }
            }
            Err(_) => {}
        }
        let display = format!(":{number}");
        let mut command = Command::new("Xvfb");
        command.args([
            &display,
            "-screen",
            "0",
            SCREEN,
            // Nothing off this machine has any business here, and a reset
            // would throw away the clipboard when the bot reconnects.
            "-nolisten",
            "tcp",
            "-noreset",
        ]);
        set_death_signal(&mut command);

        let mut server = match command.spawn() {
            Ok(server) => server,
            Err(error) => return Err(format!("cannot start Xvfb: {error}")),
        };
        if wait_for(|| Path::new(&socket).exists(), Duration::from_secs(10)) {
            return Ok((display, server));
        }
        let _ = server.kill();
        let _ = server.wait();
        failures.push(display);
    }
    Err(format!(
        "no display came up; tried {}",
        if failures.is_empty() {
            "none, every socket was taken".to_owned()
        } else {
            failures.join(", ")
        }
    ))
}

/// Asks the kernel to signal the child if we die without cleaning up.
pub fn set_death_signal(command: &mut Command) {
    use std::os::unix::process::CommandExt as _;
    unsafe {
        command.pre_exec(|| {
            rustix::process::set_parent_process_death_signal(Some(rustix::process::Signal::Term))
                .map_err(std::io::Error::from)
        });
    }
}

fn wait_for(condition: impl Fn() -> bool, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if condition() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    false
}

struct Atoms {
    clipboard: u32,
    targets: u32,
    /// Every flavour of text worth answering to, best first.
    text: Vec<u32>,
}

/// Owns the clipboard and replays keys until the sender goes away.
fn serve(
    connection: RustConnection,
    screen: usize,
    requests: Receiver<Request>,
) -> Result<(), String> {
    let root = connection.setup().roots[screen].root;
    let atoms = intern(&connection)?;
    let window = own_selection(&connection, root, atoms.clipboard)?;

    // Extensions expect to be asked for their version before they are used,
    // and asking also proves XTEST is there at all rather than finding out
    // one dropped keystroke at a time.
    match connection
        .xtest_get_version(2, 2)
        .map_err(|error| format!("XTEST is not usable: {error}"))?
        .reply()
    {
        Ok(version) => eprintln!(
            "ninjabrain-box: XTEST {}.{} on the box",
            version.major_version, version.minor_version
        ),
        Err(error) => return Err(format!("XTEST is not usable: {error}")),
    }
    let mut selection = String::new();

    loop {
        // Answering a conversion has to be prompt -- the bot is blocked on it
        // -- but nothing here is hot enough to spin over.
        match requests.recv_timeout(Duration::from_millis(10)) {
            Ok(Request::Clipboard(text)) => {
                selection = text;
                // Reclaim it each time: if anything ever took the selection
                // away, this is where we get it back.
                let _ = connection.set_selection_owner(window, atoms.clipboard, CURRENT_TIME);
                let _ = connection.flush();
            }
            Ok(Request::Key(keysym)) => match keycode(&connection, keysym) {
                Some(code) => press(&connection, code)?,
                None => eprintln!("ninjabrain-box: no key on this layout carries {keysym:#x}"),
            },
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(()),
        }

        while let Ok(Some(event)) = connection.poll_for_event() {
            if let Event::SelectionRequest(request) = event {
                answer(&connection, &atoms, &selection, request)?;
            }
        }
    }
}

fn intern(connection: &RustConnection) -> Result<Atoms, String> {
    let atom = |name: &str| -> Result<u32, String> {
        connection
            .intern_atom(false, name.as_bytes())
            .map_err(|error| format!("cannot intern {name}: {error}"))?
            .reply()
            .map(|reply| reply.atom)
            .map_err(|error| format!("cannot intern {name}: {error}"))
    };
    Ok(Atoms {
        clipboard: atom("CLIPBOARD")?,
        targets: atom("TARGETS")?,
        text: vec![
            atom("UTF8_STRING")?,
            atom("text/plain;charset=utf-8")?,
            atom("text/plain")?,
            u32::from(AtomEnum::STRING),
            atom("TEXT")?,
        ],
    })
}

/// An unmapped window to hang the selection off. Selections belong to windows,
/// and this one is never mapped, so it is never anything anybody could see.
fn own_selection(connection: &RustConnection, root: u32, clipboard: u32) -> Result<u32, String> {
    let window = connection
        .generate_id()
        .map_err(|error| format!("cannot allocate a window: {error}"))?;
    connection
        .create_window(
            x11rb::COPY_DEPTH_FROM_PARENT,
            window,
            root,
            0,
            0,
            1,
            1,
            0,
            WindowClass::INPUT_OUTPUT,
            x11rb::COPY_FROM_PARENT,
            &CreateWindowAux::new(),
        )
        .map_err(|error| format!("cannot create the selection window: {error}"))?;
    connection
        .change_window_attributes(
            window,
            &ChangeWindowAttributesAux::new().event_mask(EventMask::PROPERTY_CHANGE),
        )
        .map_err(|error| format!("cannot watch the selection window: {error}"))?;
    connection
        .set_selection_owner(window, clipboard, CURRENT_TIME)
        .map_err(|error| format!("cannot take the clipboard: {error}"))?;
    connection
        .flush()
        .map_err(|error| format!("cannot talk to the box: {error}"))?;
    Ok(window)
}

/// Hands a requestor the selection, in whichever flavour it asked for.
fn answer(
    connection: &RustConnection,
    atoms: &Atoms,
    selection: &str,
    request: xproto::SelectionRequestEvent,
) -> Result<(), String> {
    // A property of zero is an obsolete client asking us to use the target.
    let property = if request.property == 0 {
        request.target
    } else {
        request.property
    };

    let served = if request.target == atoms.targets {
        let mut targets = vec![atoms.targets];
        targets.extend_from_slice(&atoms.text);
        connection
            .change_property32(
                PropMode::REPLACE,
                request.requestor,
                property,
                AtomEnum::ATOM,
                &targets,
            )
            .is_ok()
    } else if atoms.text.contains(&request.target) {
        connection
            .change_property8(
                PropMode::REPLACE,
                request.requestor,
                property,
                request.target,
                selection.as_bytes(),
            )
            .is_ok()
    } else {
        false
    };

    let notify = SelectionNotifyEvent {
        response_type: xproto::SELECTION_NOTIFY_EVENT,
        sequence: 0,
        time: request.time,
        requestor: request.requestor,
        selection: request.selection,
        target: request.target,
        // Zero tells the requestor we could not do it.
        property: if served { property } else { 0 },
    };
    connection
        .send_event(false, request.requestor, EventMask::NO_EVENT, notify)
        .map_err(|error| format!("cannot answer a conversion: {error}"))?;
    connection
        .flush()
        .map_err(|error| format!("cannot talk to the box: {error}"))
}

/// The keycode carrying `keysym`, looked up rather than assumed: the bot reads
/// keys by their symbol, so what matters is which key on this layout produces
/// it, not what number it happens to be.
fn keycode(connection: &RustConnection, keysym: u32) -> Option<u8> {
    let setup = connection.setup();
    let (first, count) = (setup.min_keycode, setup.max_keycode - setup.min_keycode + 1);
    let mapping = connection
        .get_keyboard_mapping(first, count)
        .ok()?
        .reply()
        .ok()?;
    let per_code = mapping.keysyms_per_keycode as usize;
    mapping
        .keysyms
        .chunks(per_code)
        .position(|symbols| symbols.contains(&keysym))
        .map(|index| first + index as u8)
}

fn press(connection: &RustConnection, code: u8) -> Result<(), String> {
    for event in [xproto::KEY_PRESS_EVENT, xproto::KEY_RELEASE_EVENT] {
        // Checked rather than fired and forgotten: a void request that the
        // server rejects reports it asynchronously, and an unchecked cookie
        // throws that away -- which looks exactly like a key that did nothing.
        connection
            .xtest_fake_input(event, code, 0, x11rb::NONE, 0, 0, 0)
            .map_err(|error| format!("cannot replay a key: {error}"))?
            .check()
            .map_err(|error| format!("the box refused a key: {error}"))?;
    }
    Ok(())
}
