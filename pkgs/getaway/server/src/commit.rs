use crate::{
    Key, State,
    client::Data,
    output,
    surface::{self, Pending},
    sys::{self, Drm, Timeline},
};
use backend::OutputId;
use rustix::event::epoll::{self, EventData, EventFlags};
use std::{
    os::fd::{AsFd, OwnedFd},
    rc::Rc,
    time::Instant,
};
use wire::*;

pub type Point = (Rc<Timeline>, u64);

const MARGIN: u64 = 1_000_000;

pub struct Queued {
    pub pending: Pending,
    fences: Vec<u64>,
    wait_barrier: bool,
    not_before: Option<u64>,
}

pub fn submit(st: &mut State, key: Key, pending: Pending) {
    let fences = fences(&pending).into_iter().filter(|f| !sys::signaled(f)).filter_map(|f| watch(st, key, f)).collect();
    let (wait_barrier, not_before) = (pending.fifo_wait, pending.timestamp);
    let Some(s) = st.surfaces.get_mut(&key) else { return };
    s.queue.push_back(Queued { pending, fences, wait_barrier, not_before });
    st.queued.insert(key);
    advance(st, key);
}

fn fences(pending: &Pending) -> Vec<OwnedFd> {
    pending.subsurfaces.iter().flat_map(|(_, p)| fences(p)).chain(fence(pending)).collect()
}

fn fence(pending: &Pending) -> Option<OwnedFd> {
    if let Some((timeline, point)) = &pending.acquire {
        return timeline.wait(*point).inspect_err(|e| log::warn!("cannot wait for acquire point: {e}")).ok();
    }
    let plane = pending.attached().and_then(|b| b.dmabuf()).and_then(|d| d.planes.first())?;
    sys::pending_writes(plane.fd.as_fd())
}

fn watch(st: &mut State, key: Key, fence: OwnedFd) -> Option<u64> {
    let token = st.next_id();
    epoll::add(&st.epoll, &fence, EventData::new_u64(token), EventFlags::IN | EventFlags::ONESHOT)
        .inspect_err(|e| log::warn!("cannot watch fence, latching without it: {e}"))
        .ok()?;
    st.fences.insert(token, (key, fence));
    Some(token)
}

pub fn is_fence(st: &State, token: u64) -> bool {
    st.fences.contains_key(&token)
}

pub fn signaled(st: &mut State, token: u64) {
    let Some((key, fence)) = st.fences.remove(&token) else { return };
    let _ = epoll::delete(&st.epoll, &fence);
    if let Some(s) = st.surfaces.get_mut(&key) {
        s.queue.iter_mut().for_each(|q| q.fences.retain(|&f| f != token));
    }
    advance(st, key);
}

fn advance(st: &mut State, key: Key) {
    loop {
        let now = monotonic_ns();
        let Some(s) = st.surfaces.get(&key) else {
            st.queued.remove(&key);
            return;
        };
        let ready = s.queue.front().and_then(|q| latchable(q, s)).is_some_and(|t| latch_at(st, key, t, now) <= now);
        if !ready {
            if s.queue.is_empty() {
                st.queued.remove(&key);
            }
            return;
        }
        let q = st.surfaces.get_mut(&key).and_then(|s| s.queue.pop_front()).expect("ready implies a queued commit");
        if q.pending.fifo_barrier {
            let barrier = barrier(st, key);
            st.surfaces.get_mut(&key).expect("checked above").barrier = barrier;
        }
        st.profile.commit();
        surface::latch(st, key, q.pending);
    }
}

fn barrier(st: &State, key: Key) -> Vec<(OutputId, u64)> {
    let on = st.surfaces.get(&key).map(|s| s.outputs.as_slice()).unwrap_or_default();
    let outputs = st.outputs.iter().filter(|o| o.enabled && !o.off && (on.is_empty() || on.contains(&o.id)));
    outputs.map(|o| (o.id, o.frames + 1 + u64::from(!st.backend.ready(o.id)))).collect()
}

pub fn presented(st: &mut State, output: OutputId) {
    let frames = output::find(st, output).map_or(0, |o| o.frames);
    let live: Vec<OutputId> = st.outputs.iter().filter(|o| o.enabled && !o.off).map(|o| o.id).collect();
    let mut waiting = vec![];
    for (&key, s) in &mut st.surfaces {
        let lifted = s.barrier.iter().any(|&(o, frame)| (o == output && frames >= frame) || !live.contains(&o));
        if lifted {
            s.barrier.clear();
        }
        if lifted || (!s.queue.is_empty() && (s.outputs.is_empty() || s.outputs.contains(&output))) {
            waiting.push(key);
        }
    }
    waiting.into_iter().for_each(|k| advance(st, k));
}

fn latchable(q: &Queued, s: &surface::Surface) -> Option<u64> {
    (q.fences.is_empty() && !(q.wait_barrier && !s.barrier.is_empty())).then_some(q.not_before.unwrap_or(0))
}

fn latch_at(st: &State, key: Key, target: u64, now: u64) -> u64 {
    let Some((vblank, period)) = next_vblank(st, key, now) else { return target.saturating_sub(MARGIN) };
    match target.saturating_sub(vblank + MARGIN) {
        0 => now,
        ahead => vblank + (ahead - 1) / period * period + 1,
    }
}

fn next_vblank(st: &State, key: Key, now: u64) -> Option<(u64, u64)> {
    let o = st.surfaces.get(&key)?.outputs.first().and_then(|&o| output::find(st, o))?;
    let (vblank, period) = (o.next_vblank.filter(|_| !o.vrr)?, o.period().as_nanos() as u64);
    let at = Instant::now();
    let early = vblank.saturating_duration_since(at).as_nanos() as u64;
    let late = at.saturating_duration_since(vblank).as_nanos() as u64;
    let next = now + early + (period - late % period) % period;
    Some((next + period * u64::from(!st.backend.ready(o.id)), period))
}

fn due(st: &State) -> impl Iterator<Item = (Key, u64)> + '_ {
    let now = monotonic_ns();
    st.queued.iter().filter_map(move |&key| {
        let s = st.surfaces.get(&key)?;
        Some((key, latch_at(st, key, latchable(s.queue.front()?, s)?, now)))
    })
}

pub fn check(st: &mut State) {
    let now = monotonic_ns();
    let due: Vec<Key> = due(st).filter(|d| d.1 <= now).map(|d| d.0).collect();
    due.into_iter().for_each(|k| advance(st, k));
}

pub fn deadline(st: &State) -> Option<Instant> {
    let wait = due(st).map(|d| d.1).min()?.saturating_sub(monotonic_ns());
    Some(Instant::now() + std::time::Duration::from_nanos(wait))
}

pub fn queued(st: &State) -> usize {
    st.queued.iter().filter_map(|k| st.surfaces.get(k)).map(|s| s.queue.len()).sum()
}

pub fn forget(st: &mut State, key: Key) {
    st.queued.remove(&key);
    for (_, (_, fence)) in st.fences.extract_if(|_, f| f.0 == key) {
        let _ = epoll::delete(&st.epoll, &fence);
    }
}

fn monotonic_ns() -> u64 {
    let (sec, nsec) = crate::monotonic();
    sec * 1_000_000_000 + u64::from(nsec)
}

pub fn release(point: Option<Point>) {
    if let Some((timeline, point)) = point {
        timeline.signal(point);
    }
}

impl wp_linux_drm_syncobj_manager_v1::Handler for State {
    fn get_surface(&mut self, cx: Cx, id: Obj, surface: u32) -> Res {
        let code = wp_linux_drm_syncobj_manager_v1::error::SURFACE_EXISTS;
        surface::add_extension(self, cx, id, surface, code)?.syncobj = Some(id.id);
        Ok(())
    }

    fn import_timeline(&mut self, cx: Cx, id: Obj, fd: OwnedFd) -> Res {
        let invalid =
            |e: std::io::Error| cx.error(wp_linux_drm_syncobj_manager_v1::error::INVALID_TIMELINE, e.to_string());
        if self.drm.is_none() {
            self.drm = Some(Drm::open(self.renderer.render_node()).map_err(invalid)?);
        }
        let timeline = Timeline::import(self.drm.as_ref().expect("opened above"), &fd).map_err(invalid)?;
        self.set((cx.client, id.id), Data::Timeline(Rc::new(timeline)));
        Ok(())
    }
}

impl wp_linux_drm_syncobj_timeline_v1::Handler for State {}

fn sync_point(st: &mut State, cx: Cx, timeline: u32, hi: u32, lo: u32) -> Option<Point> {
    let Some(Data::Timeline(timeline)) = st.data_mut((cx.client, timeline)) else { return None };
    Some((timeline.clone(), (u64::from(hi) << 32) | u64::from(lo)))
}

impl wp_linux_drm_syncobj_surface_v1::Handler for State {
    fn set_acquire_point(&mut self, cx: Cx, timeline: u32, point_hi: u32, point_lo: u32) -> Res {
        let point = sync_point(self, cx, timeline, point_hi, point_lo);
        if let Some(s) = surface::extended(self, cx, Some(wp_linux_drm_syncobj_surface_v1::error::NO_SURFACE))? {
            s.pending.acquire = point;
        }
        Ok(())
    }

    fn set_release_point(&mut self, cx: Cx, timeline: u32, point_hi: u32, point_lo: u32) -> Res {
        let point = sync_point(self, cx, timeline, point_hi, point_lo);
        if let Some(s) = surface::extended(self, cx, Some(wp_linux_drm_syncobj_surface_v1::error::NO_SURFACE))? {
            s.pending.release = point;
        }
        Ok(())
    }
}

pub fn check_points(s: &surface::Surface, pending: &Pending) -> Res {
    use wp_linux_drm_syncobj_surface_v1::error::*;
    let Some(id) = s.syncobj else { return Ok(()) };
    let error = |code, message| Err(Error::new(id, code, message));
    let buffer = pending.attached();
    match (buffer, &pending.acquire, &pending.release) {
        (None, None, None) => Ok(()),
        (None, ..) => error(NO_BUFFER, "sync points without a buffer"),
        (Some(b), ..) if b.dmabuf().is_none() => error(UNSUPPORTED_BUFFER, "explicit sync needs a dmabuf"),
        (Some(_), None, _) => error(NO_ACQUIRE_POINT, "missing acquire point"),
        (Some(_), _, None) => error(NO_RELEASE_POINT, "missing release point"),
        (Some(_), Some((a, ap)), Some((r, rp))) if Rc::ptr_eq(a, r) && ap >= rp => {
            error(CONFLICTING_POINTS, "acquire point is not before release point")
        }
        _ => Ok(()),
    }
}

impl wp_fifo_manager_v1::Handler for State {
    fn get_fifo(&mut self, cx: Cx, id: Obj, surface: u32) -> Res {
        surface::add_extension(self, cx, id, surface, wp_fifo_manager_v1::error::ALREADY_EXISTS).map(drop)
    }
}

impl wp_fifo_v1::Handler for State {
    fn set_barrier(&mut self, cx: Cx) -> Res {
        if let Some(s) = surface::extended(self, cx, Some(wp_fifo_v1::error::SURFACE_DESTROYED))? {
            s.pending.fifo_barrier = true;
        }
        Ok(())
    }

    fn wait_barrier(&mut self, cx: Cx) -> Res {
        if let Some(s) = surface::extended(self, cx, Some(wp_fifo_v1::error::SURFACE_DESTROYED))? {
            s.pending.fifo_wait = true;
        }
        Ok(())
    }
}

impl wp_commit_timing_manager_v1::Handler for State {
    fn get_timer(&mut self, cx: Cx, id: Obj, surface: u32) -> Res {
        let code = wp_commit_timing_manager_v1::error::COMMIT_TIMER_EXISTS;
        surface::add_extension(self, cx, id, surface, code).map(drop)
    }
}

impl wp_commit_timer_v1::Handler for State {
    fn set_timestamp(&mut self, cx: Cx, tv_sec_hi: u32, tv_sec_lo: u32, tv_nsec: u32) -> Res {
        use wp_commit_timer_v1::error::*;
        let Some(s) = surface::extended(self, cx, Some(SURFACE_DESTROYED))? else { return Ok(()) };
        cx.error_if(s.pending.timestamp.is_some(), TIMESTAMP_EXISTS, "timestamp already set")?;
        let sec = (u64::from(tv_sec_hi) << 32) | u64::from(tv_sec_lo);
        let ns = sec.checked_mul(1_000_000_000).and_then(|ns| ns.checked_add(u64::from(tv_nsec)));
        let ns = ns.filter(|_| tv_nsec < 1_000_000_000);
        s.pending.timestamp = Some(ns.ok_or(cx.error(INVALID_TIMESTAMP, "timestamp out of range"))?);
        Ok(())
    }
}

impl wp_tearing_control_manager_v1::Handler for State {
    fn get_tearing_control(&mut self, cx: Cx, id: Obj, surface: u32) -> Res {
        let code = wp_tearing_control_manager_v1::error::TEARING_CONTROL_EXISTS;
        surface::add_extension(self, cx, id, surface, code).map(drop)
    }
}

impl wp_tearing_control_v1::Handler for State {
    fn set_presentation_hint(&mut self, cx: Cx, hint: u32) -> Res {
        if let Some(s) = surface::extended(self, cx, None)? {
            s.pending.state.tearing = Some(hint == wp_tearing_control_v1::presentation_hint::ASYNC);
        }
        Ok(())
    }

    fn destroy(&mut self, cx: Cx) -> Res {
        if let Some(s) = surface::extended(self, cx, None)? {
            s.pending.state.tearing = Some(false);
        }
        Ok(())
    }
}
