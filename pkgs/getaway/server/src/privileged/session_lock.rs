use crate::{
    ClientId, Key, State, seat,
    surface::{self, Role, configure::Configures},
    xdg,
};
use backend::OutputId;
use wire::*;

pub struct Lock {
    owner: Option<(ClientId, Obj)>,
    frames: Vec<(OutputId, u64)>,
    mapped: usize,
    announced: bool,
}

pub struct LockSurface {
    obj: Obj,
    pub output: OutputId,
    configures: Configures<(i32, i32)>,
}

impl ext_session_lock_manager_v1::Handler for State {
    fn lock(&mut self, cx: Cx, lock: Obj) {
        let c = cx.client;
        if self.lock.as_ref().is_some_and(|l| l.owner.is_some()) {
            self.send(c, |out| ext_session_lock_v1::finished(out, lock));
            return;
        }
        if let Some(l) = &mut self.lock {
            l.owner = Some((c, lock));
            if l.announced {
                self.send(c, |out| ext_session_lock_v1::locked(out, lock));
            }
            return;
        }
        let on = self.outputs.iter().filter(|o| o.enabled && !o.off);
        let frames = on.map(|o| (o.id, o.frames + 1 + u64::from(!self.backend.ready(o.id)))).collect();
        self.lock = Some(Lock { owner: Some((c, lock)), frames, mapped: 0, announced: false });
        xdg::dismiss_popups(self);
        self.render_dirty = true;
        seat::update_focus(self);
        seat::rehover(self);
        update(self);
    }
}

fn owns(st: &State, cx: Cx) -> (bool, bool) {
    let owns = st.lock.as_ref().is_some_and(|l| l.owner == Some((cx.client, cx.obj)));
    (owns, owns && st.lock.as_ref().is_some_and(|l| l.announced))
}

fn unlock(st: &mut State) {
    st.lock = None;
    for s in st.surfaces.values_mut().filter(|s| matches!(s.role, Role::LockSurface(_))) {
        s.role = Role::None;
    }
    st.render_dirty = true;
    st.shell.dirty = true;
    seat::update_focus(st);
    seat::rehover(st);
}

impl ext_session_lock_v1::Handler for State {
    fn destroy(&mut self, cx: Cx) -> Res {
        let (owns, announced) = owns(self, cx);
        if announced {
            let code = ext_session_lock_v1::error::INVALID_DESTROY;
            return Err(cx.error(code, "destroy requested after the locked event"));
        }
        if owns {
            unlock(self);
        }
        Ok(())
    }

    fn get_lock_surface(&mut self, cx: Cx, lock_surface: Obj, surface: u32, output: u32) -> Res {
        use ext_session_lock_v1::error::*;
        let c = cx.client;
        if !owns(self, cx).0 {
            return Ok(());
        }
        let output = self.client(c).output(output).or_else(|| crate::output::focused(self)).unwrap_or_default();
        if self.surfaces.values().any(|s| matches!(&s.role, Role::LockSurface(l) if l.output == output)) {
            return Err(cx.error(DUPLICATE_OUTPUT, "output already has a lock surface"));
        }
        if self.surfaces.get(&(c, surface)).is_some_and(|s| s.buffer.is_some() || s.pending.attached().is_some()) {
            return Err(cx.error(ALREADY_CONSTRUCTED, "surface already has a buffer"));
        }
        let role = Role::LockSurface(LockSurface { obj: lock_surface, output, configures: Configures::default() });
        surface::assign_role(self, (c, surface), role).map_err(|m| cx.error(ROLE, m))?;
        self.client(c).set_data(lock_surface.id, surface);
        configure(self, (c, surface));
        Ok(())
    }

    fn unlock_and_destroy(&mut self, cx: Cx) -> Res {
        if !owns(self, cx).1 {
            let code = ext_session_lock_v1::error::INVALID_UNLOCK;
            return Err(cx.error(code, "unlock requested before the locked event"));
        }
        unlock(self);
        Ok(())
    }
}

fn lock_surface(st: &mut State, key: Key) -> Option<&mut LockSurface> {
    match &mut st.surfaces.get_mut(&key)?.role {
        Role::LockSurface(l) => Some(l),
        _ => None,
    }
}

impl ext_session_lock_surface_v1::Handler for State {
    fn destroy(&mut self, cx: Cx) {
        if lock_surface(self, cx.surface()).is_some_and(|l| l.obj == cx.obj) {
            self.surfaces.get_mut(&cx.surface()).into_iter().for_each(|s| s.role = Role::None);
            update(self);
        }
    }

    fn ack_configure(&mut self, cx: Cx, serial: u32) -> Res {
        let Some(l) = lock_surface(self, cx.surface()).filter(|l| l.obj == cx.obj) else { return Ok(()) };
        let valid = l.configures.ack(serial);
        cx.error_if(!valid, ext_session_lock_surface_v1::error::INVALID_SERIAL, "invalid configure serial")
    }
}

pub fn check_commit(st: &State, key: Key, size: Option<(i32, i32)>) -> Res {
    use ext_session_lock_surface_v1::error::*;
    let Some(Role::LockSurface(s)) = st.surfaces.get(&key).map(|s| &s.role) else { return Ok(()) };
    let Some(acked) = s.configures.acked() else {
        return Err(Error::new(s.obj.id, COMMIT_BEFORE_FIRST_ACK, "commit before the first ack_configure"));
    };
    let Some(size) = size else { return Err(Error::new(s.obj.id, NULL_BUFFER, "commit with a null buffer")) };
    if s.configures.get(acked).is_some_and(|v| v != size) {
        return Err(Error::new(s.obj.id, DIMENSIONS_MISMATCH, "surface size does not match the configure"));
    }
    Ok(())
}

pub fn configure(st: &mut State, key: Key) {
    let Some(output) = lock_surface(st, key).map(|l| l.output) else { return };
    let Some(area) = crate::output::find(st, output).filter(|o| o.in_layout()).map(|o| o.rect) else { return };
    let serial = st.next_serial();
    let Some(s) = lock_surface(st, key) else { return };
    s.configures.send(serial, (area.w, area.h));
    let obj = s.obj;
    st.send(key.0, |out| ext_session_lock_surface_v1::configure(out, obj, serial, area.w as u32, area.h as u32));
}

pub fn locked(st: &State) -> bool {
    st.lock.is_some()
}

fn mapped(st: &State) -> impl Iterator<Item = (Key, OutputId)> + '_ {
    st.surfaces.iter().filter(|s| s.1.buffer.is_some()).filter_map(|(&k, s)| match &s.role {
        Role::LockSurface(l) => Some((k, l.output)),
        _ => None,
    })
}

pub fn surfaces(st: &State) -> Vec<(Key, OutputId)> {
    mapped(st).collect()
}

pub fn focus(st: &State) -> Option<Key> {
    let output = crate::output::focused(st);
    mapped(st).find(|s| Some(s.1) == output).or_else(|| mapped(st).next()).map(|s| s.0)
}

pub fn update(st: &mut State) {
    let Some(lock) = &st.lock else { return };
    let count = mapped(st).count();
    let shown = |&(id, frames): &(OutputId, u64)| {
        crate::output::find(st, id).is_none_or(|o| !o.enabled || o.off || o.frames >= frames)
    };
    let announce = !lock.announced && lock.frames.iter().all(shown);
    let remap = count != lock.mapped;
    let Some(lock) = st.lock.as_mut() else { return };
    lock.mapped = count;
    lock.announced |= announce;
    if let Some((c, obj)) = lock.owner.filter(|_| announce) {
        st.send(c, |out| ext_session_lock_v1::locked(out, obj));
    }
    if remap {
        seat::update_focus(st);
        seat::rehover(st);
    }
}

pub fn presented(st: &mut State) {
    update(st);
}

pub fn lock_gone(st: &mut State, c: ClientId, obj: Obj) {
    if let Some(lock) = st.lock.as_mut().filter(|l| l.owner == Some((c, obj))) {
        lock.owner = None;
    }
}

pub fn reconfigure(st: &mut State) {
    let keys: Vec<Key> =
        st.surfaces.iter().filter(|s| matches!(s.1.role, Role::LockSurface(_))).map(|(&k, _)| k).collect();
    keys.into_iter().for_each(|key| configure(st, key));
}
