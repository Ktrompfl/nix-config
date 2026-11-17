#![deny(unsafe_code)]
#![allow(refining_impl_trait)]

mod activation;
mod backend;
mod base;
mod buffer;
mod client;
mod commit;
mod constraints;
mod cursor;
mod dmabuf;
mod dnd;
mod gestures;
mod hints;
mod output;
mod privileged;
mod profile;
mod render;
mod scale;
mod scene;
mod seat;
mod selection;
mod shell;
mod shm;
mod surface;
mod sys;
mod xdg;
mod xwayland;

use ::backend::{Event, OutputId};
use backend::Backend;
use nested::Nested;
use rustix::{
    event::{
        Timespec,
        epoll::{self, CreateFlags, EventData, EventFlags},
    },
    fs::{FlockOperation, flock},
    io::Errno,
};
use std::{
    collections::{HashMap, HashSet},
    env,
    error::Error,
    fs::{self, File},
    io,
    mem::MaybeUninit,
    os::{fd::OwnedFd, unix::net::UnixListener},
    path::PathBuf,
    process::{Child, Command},
    rc::Rc,
    time::Instant,
};
use wire::Out;
use wm::{Cx, Policy, Rect, Request, WindowId};

pub use ::backend::OutputInfo;
pub use kms::{Accel, ClickMethod, DeviceKind, InputConfig, InputDevice, ScrollMethod};
pub use output::OutputConfig;
pub use seat::KeymapNames;

pub type ClientId = u64;
pub type Key = (ClientId, u32);
pub type Caps = u32;

pub mod caps {
    use super::Caps;
    pub const NONE: Caps = 0;
    pub const LAYER_SHELL: Caps = 1;
    pub const DATA_CONTROL: Caps = 2;
    pub const SCREENCOPY: Caps = 4;
    pub const SESSION_LOCK: Caps = 8;
    pub const IDLE_NOTIFY: Caps = 16;
    pub const FOREIGN_TOPLEVEL: Caps = 32;
    pub const WORKSPACES: Caps = 64;
    pub const VIRTUAL_INPUT: Caps = 128;
    pub const OUTPUT_MANAGEMENT: Caps = 256;
    pub const GAMMA: Caps = 512;
    pub const OUTPUT_POWER: Caps = 1024;
}

const LISTENER: u64 = 0;
const BACKEND: u64 = 1;
const SIGNALS: u64 = 2;

pub struct State {
    epoll: OwnedFd,
    socket: Socket,
    signals: OwnedFd,
    backend: Backend,
    scanout_targets: HashMap<u64, gfx::TexId>,
    outputs: Vec<output::Output>,
    output_config: fn(&OutputInfo, &[OutputInfo]) -> OutputConfig,
    policy: Box<dyn Policy>,
    clients: HashMap<ClientId, client::Client>,
    surfaces: HashMap<Key, surface::Surface>,
    windows: HashMap<WindowId, Key>,
    popups: Vec<Key>,
    layers: Vec<Key>,
    selections: [Option<Key>; 2],
    drag: Option<dnd::Drag>,
    fences: HashMap<u64, (Key, std::os::fd::OwnedFd)>,
    drm: Option<Rc<sys::Drm>>,
    profile: profile::Profile,
    management: privileged::output_management::Management,
    output_overrides: HashMap<[String; 4], output::Override>,
    capture_targets: HashMap<u64, gfx::TexId>,
    retired: Vec<surface::Retired>,
    queued: HashSet<Key>,
    cursor_cache: Option<(u64, Rc<gfx::Pixels>)>,
    lock: Option<privileged::session_lock::Lock>,
    last_input: Instant,
    tokens: HashMap<String, bool>,
    cursor: cursor::Cursor,
    cursor_theme: cursor::Theme,
    presented_feedback: Vec<(Key, OutputId)>,
    texts: HashMap<render::TextKey, render::Text>,
    xwayland: Option<xwayland::Xwayland>,
    renderer: gfx::Renderer,
    garbage: buffer::Garbage,
    format_table: dmabuf::FormatTable,
    client_caps: fn(&str) -> Caps,
    seat: seat::Seat,
    shell: shell::Shell,
    render_dirty: bool,
    presented: Vec<(Key, OutputId)>,
    next_id: u64,
    serial: u32,
    start: Instant,
    children: Vec<Child>,
    quit: bool,
}

pub struct Config {
    pub client_caps: fn(&str) -> Caps,
    pub xwayland: bool,
    pub keymap: Option<seat::KeymapNames>,
    pub input: kms::Config,
    pub outputs: fn(&OutputInfo, &[OutputInfo]) -> OutputConfig,
    pub nested_outputs: u32,
}

fn open_backend(config: &Config, renderer: &gfx::Renderer) -> Result<Backend, Box<dyn Error>> {
    match env::var_os("WAYLAND_DISPLAY") {
        Some(_) => {
            let render = (renderer.render_node(), renderer.target_formats());
            Ok(Backend::Nested(Box::new(Nested::connect(config.nested_outputs, render)?)))
        }
        None => {
            let (node, formats) = (renderer.render_node(), renderer.target_formats());
            Ok(Backend::Kms(Box::new(kms::Kms::open(config.input, node, formats, true)?)))
        }
    }
}

pub fn probe_kms(config: Config) -> Result<(), Box<dyn Error>> {
    let mut renderer = gfx::Renderer::new()?;
    let kms = kms::Kms::open(config.input, renderer.render_node(), renderer.target_formats(), false)?;
    let infos = kms.outputs();
    let wanted: Vec<OutputConfig> = infos.iter().map(|info| (config.outputs)(info, &infos)).collect();
    for (info, wanted) in infos.iter().zip(&wanted) {
        log::info!("{} ({}) {:?}, configured as {wanted:?}", info.name, info.model, info.mode);
    }
    let mut kms = kms;
    for (info, wanted) in infos.iter().zip(&wanted) {
        kms.configure(info.id, wanted.wanted_mode(), wanted.vrr, wanted.enabled)?;
        let chosen = kms.outputs().into_iter().find(|o| o.id == info.id).map(|o| o.mode);
        log::info!("{}: using {chosen:?}", info.name);
        let size = kms.size(info.id);
        let (slot, scanout) = kms.next_buffer(info.id).ok_or("no free scanout buffer")?;
        let target = renderer.import(&scanout.dmabuf, true)?;
        let ops = [gfx::Op::Fill(Rect::new(0, 0, size.0, size.1), 0xff336699)];
        let frame = gfx::Frame { size, transform: 0 };
        let fence = renderer.render(&ops, frame, gfx::Destination::Texture(target), None)?;
        log::info!("{}: cursor plane {:?}", info.name, kms.cursor_size(info.id));
        let pixels = [0xffffffff_u32; 32 * 32];
        let cursor = kms::CursorImage { key: 1, pixels: &pixels, size: (32, 32), position: (100, 100) };
        match kms.present(info.id, kms::Primary::Slot(slot, fence), Some(cursor), false) {
            Ok(()) => log::info!("{}: rendered and committed a {}x{} frame", info.name, size.0, size.1),
            Err(e) => log::info!("{}: rendered a {}x{} frame, commit refused: {e}", info.name, size.0, size.1),
        }
    }
    Ok(())
}

pub fn run(policy: Box<dyn Policy>, config: Config, autostart: &[String]) -> Result<(), Box<dyn Error>> {
    shm::install_sigbus_handler()?;
    client::raise_fd_limit();
    let renderer = gfx::Renderer::new()?;
    let backend = open_backend(&config, &renderer)?;
    let socket = Socket::bind()?;
    let epoll = epoll::create(CreateFlags::CLOEXEC)?;
    epoll::add(&epoll, &socket.listener, EventData::new_u64(LISTENER), EventFlags::IN)?;
    let signals = sys::termination()?;
    epoll::add(&epoll, &signals, EventData::new_u64(SIGNALS), EventFlags::IN)?;
    for fd in backend.fds() {
        epoll::add(&epoll, fd, EventData::new_u64(BACKEND), EventFlags::IN)?;
    }
    log::info!("WAYLAND_DISPLAY={}", socket.name);
    let mut st = State {
        epoll,
        socket,
        signals,
        backend,
        scanout_targets: HashMap::new(),
        outputs: vec![],
        output_config: config.outputs,
        policy,
        clients: HashMap::new(),
        surfaces: HashMap::new(),
        windows: HashMap::new(),
        popups: vec![],
        layers: vec![],
        selections: [None; 2],
        drag: None,
        fences: HashMap::new(),
        drm: None,
        profile: profile::Profile::from_env(),
        management: privileged::output_management::Management::default(),
        output_overrides: HashMap::new(),
        capture_targets: HashMap::new(),
        retired: vec![],
        queued: HashSet::new(),
        cursor_cache: None,
        lock: None,
        last_input: Instant::now(),
        tokens: HashMap::new(),
        cursor: cursor::Cursor::Named("default"),
        cursor_theme: Default::default(),
        presented_feedback: vec![],
        texts: HashMap::new(),
        xwayland: None,
        format_table: dmabuf::FormatTable::new(&renderer)?,
        renderer,
        garbage: Default::default(),
        client_caps: config.client_caps,
        seat: seat::Seat::default(),
        shell: shell::Shell::default(),
        render_dirty: true,
        presented: vec![],
        next_id: SIGNALS,
        serial: 0,
        start: Instant::now(),
        children: vec![],
        quit: false,
    };
    sys::realtime();
    seat::initial_keymap(&mut st, config.keymap)?;
    if config.xwayland {
        xwayland::start(&mut st);
    }
    let session = matches!(st.backend, Backend::Kms(_));
    if session {
        st.export_session_environment();
        st.uphold_graphical_session();
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            previous(info);
            end_session();
        }));
    }
    sys::notify("READY=1");
    st.spawn(autostart);
    let result = serve(&mut st);
    if session {
        end_session();
    }
    result
}

const SESSION_VARIABLES: [&str; 5] =
    ["WAYLAND_DISPLAY", "DISPLAY", "XDG_CURRENT_DESKTOP", "XDG_SESSION_TYPE", "_JAVA_AWT_WM_NONREPARENTING"];

fn end_session() {
    sys::notify("STOPPING=1");
    let status = Command::new("systemctl")
        .args(["--user", "unset-environment"])
        .args(SESSION_VARIABLES)
        .env_remove("NOTIFY_SOCKET")
        .status();
    if !status.as_ref().is_ok_and(|s| s.success()) {
        log::warn!("cannot unset the session environment: {status:?}");
    }
}

fn serve(st: &mut State) -> Result<(), Box<dyn Error>> {
    let mut events = vec![];
    let mut backend_readable = false;
    while !st.quit {
        let start = Instant::now();
        st.backend.dispatch(backend_readable, &mut events)?;
        for event in events.drain(..) {
            st.backend_event(event);
        }
        st.profile.span("backend", 0, start);
        let start = Instant::now();
        commit::check(st);
        shell::update(st);
        privileged::idle_notify::check(st);
        seat::check_repeat(st);
        if st.render_dirty || st.shell.resync {
            scale::sync(st);
        }
        st.profile.span("update", 0, start);
        render::render(st);
        let start = Instant::now();
        client::flush_all(st);
        st.profile.span("flush", 0, start);
        let counts = profile::Counts {
            clients: st.clients.len(),
            surfaces: st.surfaces.len(),
            textures: st.renderer.texture_count(),
            queued: commit::queued(st),
        };
        st.profile.tick(counts);
        backend_readable = false;
        let events = st.wait()?;
        st.profile.wakeup();
        let start = Instant::now();
        for (source, flags) in events {
            match source {
                LISTENER => client::accept(st),
                BACKEND => backend_readable = true,
                SIGNALS => {
                    let (terminate, child) = sys::signals(&st.signals);
                    if child {
                        st.children.retain_mut(|c| matches!(c.try_wait(), Ok(None)));
                        xwayland::reap(st);
                    }
                    if terminate {
                        log::info!("terminating on signal");
                        st.quit = true;
                    }
                }
                f if commit::is_fence(st, f) => commit::signaled(st, f),
                c => client::ready(st, c, flags),
            }
        }
        st.profile.span("dispatch", 0, start);
    }
    Ok(())
}

impl State {
    fn wait(&self) -> io::Result<Vec<(u64, EventFlags)>> {
        let deadlines = [
            self.shell.deadline(),
            privileged::idle_notify::deadline(self),
            seat::repeat_deadline(self),
            commit::deadline(self),
            self.profile.deadline(),
            render::deadline(self),
        ];
        let deadline = deadlines.into_iter().flatten().min();
        let timeout = deadline.map(|d| {
            let t = d.saturating_duration_since(Instant::now());
            Timespec { tv_sec: t.as_secs() as _, tv_nsec: t.subsec_nanos().into() }
        });
        let mut buf = [MaybeUninit::uninit(); 64];
        match epoll::wait(&self.epoll, &mut buf, timeout.as_ref()) {
            Ok((events, _)) => Ok(events.iter().map(|e| (e.data.u64(), e.flags)).collect()),
            Err(Errno::INTR) => Ok(vec![]),
            Err(e) => Err(e.into()),
        }
    }

    fn backend_event(&mut self, event: Event) {
        log::trace!("backend: {event:?}");
        match event {
            Event::Output(event) => output::event(self, event),
            Event::Input(event) => seat::input(self, event),
            Event::Frame { output, flip } => render::frame_done(self, output, flip),
            Event::Session(active) => seat::session(self, active),
            Event::Close => self.quit = true,
        }
    }

    fn run(&mut self, cx: Cx) {
        for request in cx.requests {
            match request {
                Request::Spawn(command) => {
                    self.spawn(&command);
                }
                Request::Close(w) => xdg::close(self, w),
                Request::Quit => self.quit = true,
                Request::SwitchVt(vt) => self.backend.switch_vt(vt),
                Request::StepVt(delta) => self.backend.step_vt(delta),
                Request::FreezeMirrors(freeze) => render::freeze_mirrors(self, freeze),
            }
        }
    }

    fn command(&self, program: &str) -> Command {
        let mut command = Command::new(program);
        command.env("WAYLAND_DISPLAY", &self.socket.name).env_remove("NOTIFY_SOCKET");
        command.env("XDG_CURRENT_DESKTOP", "getaway").env("XDG_SESSION_TYPE", "wayland");
        command.env("_JAVA_AWT_WM_NONREPARENTING", "1");
        if let Some(x) = &self.xwayland {
            command.env("DISPLAY", &x.display);
        }
        command
    }

    fn spawn(&mut self, command: &[String]) -> bool {
        let Some((program, args)) = command.split_first() else { return false };
        match self.command(program).args(args).spawn() {
            Ok(child) => {
                self.children.push(child);
                true
            }
            Err(e) => {
                log::warn!("cannot spawn {program}: {e}");
                false
            }
        }
    }

    fn export_session_environment(&self) {
        let variables = SESSION_VARIABLES.into_iter().filter(|&v| v != "DISPLAY" || self.xwayland.is_some());
        let mut command = self.command("dbus-update-activation-environment");
        let status = command.arg("--systemd").args(variables).status();
        if !status.as_ref().is_ok_and(|s| s.success()) {
            log::warn!("cannot export the session environment: {status:?}");
        }
    }

    fn uphold_graphical_session(&mut self) {
        let mut command = self.command("systemd-run");
        command.args(["--user", "--scope", "--collect", "--quiet"]);
        command.arg(format!("--unit=getaway-session-{}", std::process::id()));
        command.args(["--property=Upholds=graphical-session.target", "--property=Before=graphical-session.target"]);
        command.args(["--property=Wants=graphical-session-pre.target", "--", "sleep", "infinity"]);
        sys::die_with_parent(&mut command);
        match command.spawn() {
            Ok(child) => self.children.push(child),
            Err(e) => log::warn!("cannot start the graphical session scope: {e}"),
        }
    }

    fn send(&mut self, c: ClientId, f: impl FnOnce(&mut Out)) {
        if let Some(client) = self.clients.get_mut(&c) {
            f(&mut client.out);
        }
    }

    fn data(&self, key: Key) -> Option<&client::Data> {
        Some(&self.clients.get(&key.0)?.get(key.1)?.state)
    }

    fn data_mut(&mut self, key: Key) -> Option<&mut client::Data> {
        Some(&mut self.clients.get_mut(&key.0)?.get_mut(key.1)?.state)
    }

    fn set(&mut self, key: Key, data: client::Data) {
        if let Some(d) = self.data_mut(key) {
            *d = data;
        }
    }

    fn objects(&self, caps: Caps) -> impl Iterator<Item = (ClientId, &wire::Object<client::Data>)> {
        let clients = self.clients.iter().filter(move |c| c.1.caps & caps == caps);
        clients.flat_map(|(&c, client)| client.objects().map(move |o| (c, o)))
    }

    fn each_object(&mut self, caps: Caps, mut f: impl FnMut(ClientId, &mut Out, &mut wire::Object<client::Data>)) {
        for (&c, client) in self.clients.iter_mut().filter(|c| c.1.caps & caps == caps) {
            client.each_mut(|out, o| f(c, out, o));
        }
    }

    fn client(&mut self, c: ClientId) -> &mut client::Client {
        self.clients.get_mut(&c).expect("dispatching client exists")
    }

    fn next_serial(&mut self) -> u32 {
        self.serial = self.serial.wrapping_add(1);
        self.serial
    }

    fn next_id(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }

    fn time(&self) -> u32 {
        self.start.elapsed().as_millis() as u32
    }
}

impl wire::Server for State {
    type Data = client::Data;

    fn connection(&mut self, c: ClientId) -> &mut wire::Connection<client::Data> {
        self.client(c)
    }
}

struct Socket {
    listener: UnixListener,
    name: String,
    path: PathBuf,
    _lock: File,
}

impl Socket {
    fn bind() -> io::Result<Self> {
        let dir = PathBuf::from(env::var_os("XDG_RUNTIME_DIR").ok_or(io::Error::other("XDG_RUNTIME_DIR is not set"))?);
        for n in 1..32 {
            let name = format!("wayland-{n}");
            let path = dir.join(&name);
            let lock = File::options().create(true).truncate(false).write(true).open(path.with_extension("lock"))?;
            if flock(&lock, FlockOperation::NonBlockingLockExclusive).is_err() {
                continue;
            }
            let _ = fs::remove_file(&path);
            let listener = UnixListener::bind(&path)?;
            listener.set_nonblocking(true)?;
            return Ok(Self { listener, name, path, _lock: lock });
        }
        Err(io::Error::other("no free wayland socket"))
    }
}

impl Drop for Socket {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
        let _ = fs::remove_file(self.path.with_extension("lock"));
    }
}

fn monotonic() -> (u64, u32) {
    let now = rustix::time::clock_gettime(rustix::time::ClockId::Monotonic);
    (now.tv_sec as u64, now.tv_nsec as u32)
}
