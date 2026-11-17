use crate::{
    Caps, ClientId, State,
    buffer::Buffer,
    constraints, dmabuf, dnd,
    privileged::{
        capture, foreign_toplevel, gamma, idle_notify, output_management, session_lock, virtual_input, workspaces,
    },
    scale, seat, selection,
    shm::Pool,
    surface,
    sys::Timeline,
    xdg,
};
use backend::OutputId;
use rustix::{
    event::epoll::{self, EventData, EventFlags},
    net::sockopt::socket_peercred,
};
use std::{
    cell::Cell,
    fs,
    ops::{Deref, DerefMut},
    os::unix::net::UnixStream,
    rc::Rc,
};
use wire::*;

pub struct Client {
    pub conn: Connection<Data>,
    pub caps: Caps,
    pub keymap: Option<Option<crate::Key>>,
    pub pools: Rc<Cell<usize>>,
}

#[derive(Default)]
pub enum Data {
    #[default]
    None,
    Region(geom::Region),
    Positioner(xdg::Positioner),
    Pool(Rc<Pool>),
    Buffer(Rc<Buffer>),
    Params(dmabuf::Params),
    Timeline(Rc<Timeline>),
    Output(Option<OutputId>, Option<scale::Shown>),
    XdgOutput(u32, Option<wm::Rect>),
    Token(Option<u32>, bool),
    Source(selection::Source),
    Offer(selection::Offer),
    ToplevelDrag(u32),
    VirtualKeyboard(virtual_input::VirtualKeyboard),
    VirtualPointer(virtual_input::VirtualPointer),
    Gamma(OutputId, u32),
    Power(OutputId),
    Notification(idle_notify::Notification),
    CaptureSource(capture::Source),
    Session(capture::Session),
    Frame(capture::Frame),
    CursorSession,
    Toplevels(Vec<u64>),
    Handle(foreign_toplevel::Handle),
    Head(output_management::Head),
    Config(output_management::Configuration),
    ConfigHead(u32, OutputId),
    Workspaces(workspaces::Manager),
    WorkspaceGroup(workspaces::Group),
    Workspace(workspaces::Workspace),
}

impl Client {
    fn new(stream: UnixStream, caps: Caps) -> Self {
        Self { conn: Connection::new(stream), caps, keymap: None, pools: Default::default() }
    }

    pub fn output(&self, id: u32) -> Option<OutputId> {
        match self.get_of(id, Interface::WlOutput)?.state {
            Data::Output(output, _) => output,
            _ => None,
        }
    }

    pub fn outputs(&self) -> Vec<(Obj, Option<OutputId>)> {
        let outputs = self.objects().filter_map(|o| match o.state {
            Data::Output(output, _) => Some((o.obj, output)),
            _ => None,
        });
        outputs.collect()
    }
}

pub fn enter(
    entered: &mut Vec<u32>,
    all: &[(Obj, Option<OutputId>)],
    on: &[OutputId],
    mut f: impl FnMut(u32, bool),
) -> bool {
    let old = std::mem::take(entered);
    let mut changed = false;
    for &(output, id) in all {
        let wanted = id.is_some_and(|id| on.contains(&id));
        if wanted != old.contains(&output.id) {
            f(output.id, wanted);
            changed = true;
        }
        if wanted {
            entered.push(output.id);
        }
    }
    changed
}

impl Deref for Client {
    type Target = Connection<Data>;

    fn deref(&self) -> &Connection<Data> {
        &self.conn
    }
}

impl DerefMut for Client {
    fn deref_mut(&mut self) -> &mut Connection<Data> {
        &mut self.conn
    }
}

pub fn accept(st: &mut State) {
    while let Ok((stream, _)) = st.socket.listener.accept() {
        let c = st.next_id();
        if stream.set_nonblocking(true).is_err()
            || epoll::add(&st.epoll, &stream, EventData::new_u64(c), EventFlags::IN).is_err()
        {
            continue;
        }
        let exe = executable(&stream).unwrap_or_default();
        let caps = (st.client_caps)(&exe);
        st.clients.insert(c, Client::new(stream, caps));
        log::debug!("client {c} connected: {exe} caps {caps:#x}");
    }
}

fn executable(stream: &UnixStream) -> Option<String> {
    let pid = socket_peercred(stream).ok()?.pid.as_raw_nonzero();
    let path = fs::read_link(format!("/proc/{pid}/exe")).ok()?;
    let name = path.file_name()?.to_str()?;
    Some(name.strip_prefix('.').and_then(|n| n.strip_suffix("-wrapped")).unwrap_or(name).to_string())
}

pub fn ready(st: &mut State, c: ClientId, flags: EventFlags) {
    if !flags.intersects(EventFlags::IN | EventFlags::HUP | EventFlags::ERR) {
        return;
    }
    let Some(client) = st.clients.get_mut(&c) else { return };
    let Ok(mut input) = client.receive() else { return disconnect(st, c) };
    match process(st, c, &mut input) {
        Ok(()) => st.client(c).restore(input),
        Err(e) => fail(st, c, e),
    }
}

fn process(st: &mut State, c: ClientId, input: &mut Input) -> Res {
    let malformed = |e| Error::new(DISPLAY.id, wl_display::error::INVALID_METHOD, format!("{e:?}"));
    while let Some(mut msg) = input.message().map_err(malformed)? {
        if wire::dispatch(st, c, &mut msg)? {
            remove_object(st, c, msg.id);
        }
    }
    Ok(())
}

pub fn send_destructor(st: &mut State, c: ClientId, obj: Obj, event: impl FnOnce(&mut Out, Obj)) {
    st.send(c, |out| event(out, obj));
    remove_object(st, c, obj.id);
}

pub fn raise_fd_limit() {
    use rustix::process::{Resource, getrlimit, setrlimit};
    let mut limit = getrlimit(Resource::Nofile);
    if limit.current < limit.maximum {
        limit.current = limit.maximum;
        if let Err(e) = setrlimit(Resource::Nofile, limit) {
            log::warn!("cannot raise the descriptor limit: {e}");
        }
    }
}

pub fn remove_object(st: &mut State, c: ClientId, id: u32) {
    if let Some(object) = st.clients.get_mut(&c).and_then(|client| client.delete(id)) {
        destroyed(st, c, object);
    }
}

fn destroyed(st: &mut State, c: ClientId, o: Object<Data>) {
    let (key, surface) = ((c, o.obj.id), (c, o.data));
    match (o.interface, o.state) {
        (Interface::WlSurface, _) => surface::destroy(st, key),
        (Interface::WlOutput, _) => st.shell.resync = true,
        (Interface::ZwpLockedPointerV1 | Interface::ZwpConfinedPointerV1, _) => constraints::gone(st, surface, o.obj),
        (Interface::ZwpIdleInhibitorV1 | Interface::WpFractionalScaleV1 | Interface::WpLinuxDrmSyncobjSurfaceV1, _) => {
            surface::extension_gone(st, surface, o.interface)
        }
        (Interface::ZwpVirtualKeyboardV1, _) => seat::virtual_keyboard_gone(st, key),
        (Interface::ExtSessionLockV1, _) => session_lock::lock_gone(st, c, o.obj),
        (_, Data::Source(_)) => selection::source_gone(st, key),
        (_, Data::Offer(offer)) => dnd::offer_gone(st, o.obj, offer),
        (_, Data::Gamma(output, _)) => gamma::set(st, output, None),
        (_, Data::Session(session)) => capture::session_gone(st, c, session),
        (_, Data::Frame(frame)) => capture::frame_gone(st, key, frame),
        (_, Data::Config(_)) => output_management::config_gone(st, key),
        _ => {}
    }
}

pub fn fail(st: &mut State, c: ClientId, e: Error) {
    log::warn!("client {c}: protocol error on object {}: {}", e.object, e.message);
    if let Some(client) = st.clients.get_mut(&c) {
        client.post(&e);
    }
    disconnect(st, c);
}

pub fn disconnect(st: &mut State, c: ClientId) {
    let Some(mut client) = st.clients.remove(&c) else { return };
    let _ = epoll::delete(&st.epoll, &client.conn);
    for object in client.take_all() {
        destroyed(st, c, object);
    }
    dnd::client_gone(st, c);
    log::debug!("client {c} disconnected");
}

pub fn flush_all(st: &mut State) {
    let mut dead = vec![];
    for (&c, client) in &mut st.clients {
        match client.flush() {
            Ok(None) => {}
            Ok(Some(writing)) => {
                let flags = if writing { EventFlags::IN | EventFlags::OUT } else { EventFlags::IN };
                let _ = epoll::modify(&st.epoll, &client.conn, EventData::new_u64(c), flags);
            }
            Err(_) => dead.push(c),
        }
    }
    for c in dead {
        disconnect(st, c);
    }
}
