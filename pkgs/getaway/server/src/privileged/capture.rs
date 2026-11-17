use crate::{
    ClientId, Key, State,
    buffer::Buffer,
    caps,
    client::Data,
    output,
    privileged::{foreign_toplevel, session_lock},
    render,
};
use backend::OutputId;
use gfx::Op;
use std::rc::Rc;
use wire::*;
use wm::WindowId;

const FORMATS: [(u32, u32); 2] = [(0x3432_5258, wl_shm::format::XRGB8888), (0x3432_5241, wl_shm::format::ARGB8888)];

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Source {
    Output(OutputId),
    Window(WindowId),
    Gone,
}

pub struct Session {
    obj: Obj,
    source: Source,
    cursors: bool,
    size: (i32, i32),
    fresh: bool,
    damaged: bool,
    seen: u64,
    frame: Option<u32>,
}

pub struct Frame {
    session: u32,
    obj: Obj,
    buffer: Option<Rc<Buffer>>,
    captured: bool,
}

impl ext_output_image_capture_source_manager_v1::Handler for State {
    fn create_source(&mut self, cx: Cx, source: Obj, output: u32) {
        let found = self.client(cx.client).output(output).map_or(Source::Gone, Source::Output);
        self.set((cx.client, source.id), Data::CaptureSource(found));
    }
}

impl ext_foreign_toplevel_image_capture_source_manager_v1::Handler for State {
    fn create_source(&mut self, cx: Cx, source: Obj, toplevel_handle: u32) {
        let window = foreign_toplevel::window_of(self, (cx.client, toplevel_handle));
        self.set((cx.client, source.id), Data::CaptureSource(window.map_or(Source::Gone, Source::Window)));
    }
}

impl ext_image_capture_source_v1::Handler for State {}

fn session(st: &State, key: Key) -> Option<&Session> {
    match st.data(key)? {
        Data::Session(s) => Some(s),
        _ => None,
    }
}

fn session_mut(st: &mut State, key: Key) -> Option<&mut Session> {
    match st.data_mut(key)? {
        Data::Session(s) => Some(s),
        _ => None,
    }
}

fn frame(st: &State, key: Key) -> Option<&Frame> {
    match st.data(key)? {
        Data::Frame(f) => Some(f),
        _ => None,
    }
}

fn sessions(st: &State, f: impl Fn(&State, &Session) -> bool) -> Vec<Key> {
    let sessions = st.objects(caps::SCREENCOPY).filter_map(|(c, o)| match &o.state {
        Data::Session(s) if f(st, s) => Some((c, o.obj.id)),
        _ => None,
    });
    sessions.collect()
}

impl ext_image_copy_capture_manager_v1::Handler for State {
    fn create_session(&mut self, cx: Cx, session: Obj, source: u32, options: u32) -> Res {
        use ext_image_copy_capture_manager_v1::options::PAINT_CURSORS;
        if options & !PAINT_CURSORS != 0 {
            return Err(cx.error(ext_image_copy_capture_manager_v1::error::INVALID_OPTION, "unknown capture options"));
        }
        let source = match self.data((cx.client, source)) {
            Some(&Data::CaptureSource(source)) => source,
            _ => Source::Gone,
        };
        create_session(self, cx, session, source, options & PAINT_CURSORS != 0);
        Ok(())
    }

    fn create_pointer_cursor_session(&mut self, _: Cx, _: Obj, _: u32, _: u32) {}
}

impl ext_image_copy_capture_cursor_session_v1::Handler for State {
    fn get_capture_session(&mut self, cx: Cx, session: Obj) -> Res {
        let previous = self.data_mut(cx.key()).map(|d| std::mem::replace(d, Data::CursorSession));
        if matches!(previous, Some(Data::CursorSession)) {
            let code = ext_image_copy_capture_cursor_session_v1::error::DUPLICATE_SESSION;
            return Err(cx.error(code, "capture session already created"));
        }
        create_session(self, cx, session, Source::Gone, false);
        Ok(())
    }
}

fn create_session(st: &mut State, cx: Cx, obj: Obj, source: Source, cursors: bool) {
    let session = Session { obj, source, cursors, size: (0, 0), fresh: true, damaged: false, seen: 0, frame: None };
    st.set((cx.client, obj.id), Data::Session(session));
    constrain(st, (cx.client, obj.id));
}

fn source_size(st: &State, source: Source) -> Option<(i32, i32)> {
    match source {
        Source::Output(o) => output::find(st, o).filter(|o| o.enabled).map(|o| o.frame().logical_size()),
        Source::Window(w) => {
            let s = st.surfaces.get(st.windows.get(&w)?)?;
            let g = Some(s.geometry).filter(|g| !g.is_empty()).unwrap_or(wm::Rect::new(0, 0, s.size().0, s.size().1));
            let size = g.scaled(s.preferred_scale);
            Some((size.w, size.h))
        }
        Source::Gone => None,
    }
}

fn constrain(st: &mut State, key: Key) {
    let Some(&Session { obj, source, .. }) = session(st, key) else { return };
    let Some((w, h)) = source_size(st, source) else {
        session_mut(st, key).into_iter().for_each(|s| s.source = Source::Gone);
        st.send(key.0, |out| ext_image_copy_capture_session_v1::stopped(out, obj));
        return fail_pending(st, key, ext_image_copy_capture_frame_v1::failure_reason::STOPPED);
    };
    session_mut(st, key).into_iter().for_each(|s| s.size = (w, h));
    let device = st.renderer.render_node().to_ne_bytes();
    let modifiers = |format: u32| -> Vec<u8> {
        let formats = st.renderer.target_formats().iter().filter(|f| f.0 == format);
        formats.flat_map(|f| f.1.to_ne_bytes()).collect()
    };
    let formats: Vec<(u32, u32, Vec<u8>)> = FORMATS.iter().map(|&(drm, shm)| (drm, shm, modifiers(drm))).collect();
    st.send(key.0, |out| {
        ext_image_copy_capture_session_v1::buffer_size(out, obj, w as u32, h as u32);
        for (drm, shm, modifiers) in &formats {
            ext_image_copy_capture_session_v1::shm_format(out, obj, *shm);
            if !modifiers.is_empty() {
                ext_image_copy_capture_session_v1::dmabuf_format(out, obj, *drm, modifiers);
            }
        }
        ext_image_copy_capture_session_v1::dmabuf_device(out, obj, &device);
        ext_image_copy_capture_session_v1::done(out, obj);
    });
}

impl ext_image_copy_capture_session_v1::Handler for State {
    fn create_frame(&mut self, cx: Cx, frame: Obj) -> Res {
        let Some(s) = session_mut(self, cx.key()) else { return Ok(()) };
        if s.frame.replace(frame.id).is_some() {
            let code = ext_image_copy_capture_session_v1::error::DUPLICATE_FRAME;
            return Err(cx.error(code, "a frame is already pending"));
        }
        let frame_state = Frame { session: cx.obj.id, obj: frame, buffer: None, captured: false };
        self.set((cx.client, frame.id), Data::Frame(frame_state));
        Ok(())
    }
}

pub fn session_gone(st: &mut State, c: ClientId, session: Session) {
    if let Some(frame) = session.frame {
        fail_frame(st, (c, frame), ext_image_copy_capture_frame_v1::failure_reason::STOPPED);
    }
}

pub fn frame_gone(st: &mut State, key: Key, frame: Frame) {
    if let Some(s) = session_mut(st, (key.0, frame.session)).filter(|s| s.frame == Some(key.1)) {
        s.frame = None;
    }
}

fn uncaptured(st: &mut State, cx: Cx) -> Result<Option<&mut Frame>, Error> {
    match st.data_mut(cx.key()) {
        Some(Data::Frame(f)) if f.captured => {
            Err(cx.error(ext_image_copy_capture_frame_v1::error::ALREADY_CAPTURED, "frame was already captured"))
        }
        Some(Data::Frame(f)) => Ok(Some(f)),
        _ => Ok(None),
    }
}

impl ext_image_copy_capture_frame_v1::Handler for State {
    fn attach_buffer(&mut self, cx: Cx, buffer: u32) -> Res {
        let buffer = match self.data((cx.client, buffer)) {
            Some(Data::Buffer(b)) => Some(b.clone()),
            _ => None,
        };
        if let Some(frame) = uncaptured(self, cx)? {
            frame.buffer = buffer;
        }
        Ok(())
    }

    fn damage_buffer(&mut self, cx: Cx, x: i32, y: i32, width: i32, height: i32) -> Res {
        if uncaptured(self, cx)?.is_some() && (x < 0 || y < 0 || width <= 0 || height <= 0) {
            return Err(cx.error(ext_image_copy_capture_frame_v1::error::INVALID_BUFFER_DAMAGE, "invalid damage"));
        }
        Ok(())
    }

    fn capture(&mut self, cx: Cx) -> Res {
        use ext_image_copy_capture_frame_v1::failure_reason::{BUFFER_CONSTRAINTS, STOPPED};
        let Some(frame) = uncaptured(self, cx)? else { return Ok(()) };
        let Some(buffer) = frame.buffer.clone() else {
            return Err(cx.error(ext_image_copy_capture_frame_v1::error::NO_BUFFER, "no buffer attached"));
        };
        frame.captured = true;
        let owner = (cx.client, frame.session);
        let reason = match session(self, owner).filter(|s| s.frame == Some(cx.obj.id)).map(|s| (s.source, s.size)) {
            None | Some((Source::Gone, _)) => Some(STOPPED),
            Some((_, size)) if !fits(self, &buffer, size) => Some(BUFFER_CONSTRAINTS),
            Some(_) => None,
        };
        match reason {
            Some(reason) => finish(self, cx.key(), Err(reason)),
            None => self.render_dirty = true,
        }
        Ok(())
    }
}

fn fits(st: &State, buffer: &Buffer, size: (i32, i32)) -> bool {
    let formats = FORMATS.map(|f| f.0);
    buffer.size() == size
        && match (buffer.shm(), buffer.dmabuf()) {
            (Some(shm), _) => shm.stride() == size.0 as usize * 4,
            (_, Some(dmabuf)) => {
                formats.contains(&dmabuf.format)
                    && st.renderer.target_formats().contains(&(dmabuf.format, dmabuf.modifier))
                    && dmabuf.planes.len() == 1
            }
            _ => false,
        }
}

fn pending(st: &State, session: Key) -> Option<(Key, Rc<Buffer>)> {
    let key = (session.0, self::session(st, session)?.frame?);
    let frame = frame(st, key).filter(|f| f.captured)?;
    Some((key, frame.buffer.clone()?))
}

fn finish(st: &mut State, key: Key, result: Result<(), u32>) {
    let Some(Data::Frame(frame)) = st.data_mut(key) else { return };
    let (obj, session) = (frame.obj, frame.session);
    frame.buffer = None;
    let size = self::session(st, (key.0, session)).map(|s| s.size);
    let (sec, nsec) = crate::monotonic();
    st.send(key.0, |out| match result {
        Ok(()) => {
            let (w, h) = size.unwrap_or_default();
            ext_image_copy_capture_frame_v1::transform(out, obj, wl_output::transform::NORMAL);
            ext_image_copy_capture_frame_v1::damage(out, obj, 0, 0, w, h);
            ext_image_copy_capture_frame_v1::presentation_time(out, obj, (sec >> 32) as u32, sec as u32, nsec);
            ext_image_copy_capture_frame_v1::ready(out, obj);
        }
        Err(reason) => ext_image_copy_capture_frame_v1::failed(out, obj, reason),
    });
}

fn fail_frame(st: &mut State, frame: Key, reason: u32) {
    if self::frame(st, frame).is_some_and(|f| f.captured && f.buffer.is_some()) {
        finish(st, frame, Err(reason));
    }
}

fn fail_pending(st: &mut State, session: Key, reason: u32) {
    if let Some((frame, _)) = pending(st, session) {
        finish(st, frame, Err(reason));
    }
}

fn deliver(st: &mut State, session: Key, ops: &[Op], size: (i32, i32)) {
    let Some((frame, buffer)) = pending(st, session) else { return };
    if self::session(st, session).is_some_and(|s| s.size != size) {
        constrain(st, session);
        return finish(st, frame, Err(ext_image_copy_capture_frame_v1::failure_reason::BUFFER_CONSTRAINTS));
    }
    let drawn = render::draw_into(st, ops, size, &buffer);
    if let Some(s) = session_mut(st, session) {
        (s.fresh, s.damaged) = (false, false);
    }
    let failed = ext_image_copy_capture_frame_v1::failure_reason::UNKNOWN;
    finish(st, frame, if drawn { Ok(()) } else { Err(failed) });
}

fn due(st: &State, key: Key) -> bool {
    session(st, key).is_some_and(|s| s.fresh || s.damaged) && pending(st, key).is_some()
}

pub fn wants(st: &State, output: OutputId) -> bool {
    sessions(st, |_, s| s.source == Source::Output(output)).into_iter().any(|k| due(st, k))
}

pub fn output_frames(
    st: &mut State,
    output: OutputId,
    size: (i32, i32),
    (scene, pointer): (&[Op], &[Op]),
    damaged: bool,
    cursor_changed: bool,
) {
    for key in sessions(st, |_, s| s.source == Source::Output(output)) {
        let Some(s) = session_mut(st, key) else { continue };
        s.damaged |= damaged || (s.cursors && cursor_changed);
        let cursors = s.cursors;
        if due(st, key) {
            let ops: Vec<Op> = scene.iter().chain(pointer.iter().filter(|_| cursors)).copied().collect();
            deliver(st, key, &ops, size);
        }
    }
}

pub fn window_frames(st: &mut State) {
    if session_lock::locked(st) {
        return;
    }
    for key in sessions(st, |_, s| matches!(s.source, Source::Window(_))) {
        let Some(&Session { source: Source::Window(window), fresh, seen, .. }) = session(st, key) else { continue };
        let Some(&surface) = st.windows.get(&window) else {
            constrain(st, key);
            continue;
        };
        let generation = content(st, surface);
        if !(fresh || seen != generation) || pending(st, key).is_none() {
            continue;
        }
        let scale = st.surfaces.get(&surface).map_or(1.0, |s| s.preferred_scale);
        if let Some((ops, size)) = render::window_ops(st, surface, scale) {
            session_mut(st, key).into_iter().for_each(|s| s.seen = generation);
            deliver(st, key, &ops, size);
        }
    }
}

fn content(st: &State, key: Key) -> u64 {
    let Some(s) = st.surfaces.get(&key) else { return 0 };
    s.children.iter().fold(s.commits, |sum, &child| sum.wrapping_add(content(st, (key.0, child))))
}

pub fn outputs_changed(st: &mut State) {
    let gone = sessions(st, |st, s| matches!(s.source, Source::Output(_)) && source_size(st, s.source).is_none());
    gone.into_iter().for_each(|k| constrain(st, k));
}

pub fn source_gone(st: &mut State, source: Source) {
    sessions(st, |_, s| s.source == source).into_iter().for_each(|k| constrain(st, k));
}

pub fn toplevel_closed(st: &mut State, w: WindowId) {
    let (gone, mut stopped) = (Source::Window(w), vec![]);
    st.each_object(caps::SCREENCOPY, |c, _, o| match &mut o.state {
        Data::CaptureSource(s) if *s == gone => *s = Source::Gone,
        Data::Session(s) if s.source == gone => {
            s.source = Source::Gone;
            stopped.push((c, o.obj.id));
        }
        _ => {}
    });
    stopped.into_iter().for_each(|k| constrain(st, k));
}
