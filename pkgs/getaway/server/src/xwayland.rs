use crate::{State, sys};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    os::{
        fd::AsRawFd,
        linux::net::SocketAddrExt,
        unix::{
            fs::{OpenOptionsExt, PermissionsExt},
            net::{SocketAddr, UnixListener},
        },
    },
    process::Child,
    time::{Duration, Instant},
};

const DIR: &str = "/tmp/.X11-unix";
const QUICK: Duration = Duration::from_secs(10);

pub struct Xwayland {
    pub display: String,
    sockets: [UnixListener; 2],
    files: [String; 2],
    child: Option<Child>,
    started: Instant,
    failures: u32,
}

impl Drop for Xwayland {
    fn drop(&mut self) {
        if let Some(child) = &mut self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.files.iter().for_each(|f| drop(fs::remove_file(f)));
    }
}

fn claim(n: u32) -> Option<Xwayland> {
    let lock = format!("/tmp/.X{n}-lock");
    let mut file = OpenOptions::new().write(true).create_new(true).mode(0o444).open(&lock).ok()?;
    let _ = writeln!(file, "{:>10}", std::process::id());
    let path = format!("{DIR}/X{n}");
    let bind = || {
        let abstract_socket = UnixListener::bind_addr(&SocketAddr::from_abstract_name(path.as_bytes())?)?;
        let _ = fs::remove_file(&path);
        Ok::<_, std::io::Error>([abstract_socket, UnixListener::bind(&path)?])
    };
    match bind() {
        Ok(sockets) => {
            let (display, files) = (format!(":{n}"), [lock, path]);
            Some(Xwayland { display, sockets, files, child: None, started: Instant::now(), failures: 0 })
        }
        Err(_) => {
            let _ = fs::remove_file(&lock);
            None
        }
    }
}

pub fn start(st: &mut State) {
    if fs::create_dir(DIR).is_ok() {
        let _ = fs::set_permissions(DIR, fs::Permissions::from_mode(0o1777));
    }
    let Some(xwayland) = (0..32).find_map(claim) else { return log::warn!("no free X display") };
    log::info!("DISPLAY={}", xwayland.display);
    st.xwayland = Some(xwayland);
    spawn(st);
}

fn spawn(st: &mut State) {
    let mut command = st.command("xwayland-satellite");
    let Some(x) = &mut st.xwayland else { return };
    let fds = x.sockets.each_ref().map(|s| s.as_raw_fd());
    command.arg(&x.display).env_remove("DISPLAY").env_remove("RUST_BACKTRACE").env_remove("RUST_LIB_BACKTRACE");
    for fd in fds {
        command.arg("-listenfd").arg(fd.to_string());
    }
    sys::inherit(&mut command, fds);
    match command.spawn() {
        Ok(child) => (x.child, x.started) = (Some(child), Instant::now()),
        Err(e) => {
            log::warn!("cannot spawn xwayland-satellite: {e}");
            st.xwayland = None;
        }
    }
}

pub fn reap(st: &mut State) {
    let Some(x) = &mut st.xwayland else { return };
    let Some(status) = x.child.as_mut().and_then(|c| c.try_wait().ok().flatten()) else { return };
    x.child = None;
    x.failures = if x.started.elapsed() < QUICK { x.failures + 1 } else { 0 };
    if x.failures >= 3 {
        return log::error!("xwayland-satellite keeps exiting ({status}), giving up");
    }
    log::warn!("xwayland-satellite exited ({status}), restarting");
    spawn(st);
}
