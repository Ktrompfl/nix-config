use crate::{
    Key, State,
    buffer::Buffer,
    privileged::session_lock,
    seat,
    surface::{self, Pending, Role, configure},
    xdg,
};
use std::{
    collections::{HashMap, HashSet},
    mem,
    rc::Rc,
    time::{Duration, Instant},
};
use wm::{Arrangement, WindowId};

const TRANSACTION_TIMEOUT: Duration = Duration::from_millis(100);

#[derive(Default)]
pub struct Shell {
    pub dirty: bool,
    pub target: Arrangement,
    pub shown: Arrangement,
    pub ghosts: HashMap<WindowId, Rc<Buffer>>,
    changed: bool,
    pub resync: bool,
    transaction: Option<Transaction>,
}

struct Transaction {
    waiting: HashMap<WindowId, u32>,
    deadline: Instant,
}

impl Shell {
    pub fn deadline(&self) -> Option<Instant> {
        self.transaction.as_ref().map(|t| t.deadline)
    }
}

pub fn update(st: &mut State) {
    if mem::take(&mut st.shell.dirty) {
        arrange(st);
        st.shell.resync = true;
    }
    session_lock::update(st);
    seat::update_focus(st);
    let settled = st.shell.transaction.as_ref().is_none_or(|t| t.waiting.is_empty() || Instant::now() >= t.deadline);
    if settled && (st.shell.transaction.is_some() || st.shell.changed) {
        finish(st);
    }
}

fn arrange(st: &mut State) {
    let mut next = Arrangement::default();
    st.policy.wm().arrange(&mut next);
    for p in &next.windows {
        if let Some(serial) = xdg::configure(st, p) {
            let deadline = Instant::now() + TRANSACTION_TIMEOUT;
            let transaction =
                st.shell.transaction.get_or_insert_with(|| Transaction { waiting: HashMap::new(), deadline });
            transaction.waiting.insert(p.window, serial);
        }
    }
    let placed: HashSet<WindowId> = next.windows.iter().map(|p| p.window).collect();
    for window in xdg::managed_windows(st).map(|w| w.0).filter(|w| !placed.contains(w)).collect::<Vec<_>>() {
        xdg::suspend(st, window);
    }
    if next.focus != st.shell.target.focus && !st.popups.is_empty() {
        xdg::dismiss_popups(st);
    }
    if next == st.shell.target {
        return;
    }
    st.shell.changed = true;
    st.shell.target = next;
}

fn finish(st: &mut State) {
    (st.shell.transaction, st.shell.changed) = (None, false);
    let held: Vec<Key> = st.surfaces.iter().filter(|(_, s)| s.held.is_some()).map(|(k, _)| *k).collect();
    for key in held {
        if let Some(pending) = st.surfaces.get_mut(&key).and_then(|s| s.held.take()) {
            surface::apply(st, key, pending);
        }
    }
    let (old, new) = (&st.shell.shown, &st.shell.target);
    let refocused = old.focus != new.focus;
    let away = |r: wm::Rect| st.seat.pointer().is_none_or(|(x, y)| !r.contains(x, y));
    let warp = old.warps != new.warps || (refocused && new.pointer.is_some_and(away));
    st.shell.shown = st.shell.target.clone();
    st.shell.ghosts.clear();
    st.render_dirty = true;
    match st.shell.shown.pointer.filter(|_| warp && !session_lock::locked(st)) {
        Some(r) => seat::warp_to_center(st, r),
        None => seat::rehover(st),
    }
}

pub fn hold(st: &mut State, key: Key, pending: Pending) -> Option<Pending> {
    let surface = st.surfaces.get_mut(&key)?;
    let (Role::Toplevel(t), Some(transaction)) = (&surface.role, &mut st.shell.transaction) else {
        return Some(pending);
    };
    let (window, acked) = (t.window(), pending.state.configure.unwrap_or(surface.state.configure));
    if transaction.waiting.get(&window).is_some_and(|&serial| acked.is_some_and(|a| configure::reached(a, serial))) {
        transaction.waiting.remove(&window);
    } else if surface.held.is_none() {
        return Some(pending);
    }
    let mut held = surface.held.take().unwrap_or_default();
    surface::merge(st, key.0, &mut held, pending);
    st.surfaces.get_mut(&key)?.held = Some(held);
    None
}

pub fn detached(st: &mut State, w: WindowId, buffer: Rc<Buffer>) {
    xdg::withdraw(st, w, Some(buffer));
}

pub fn window_gone(st: &mut State, w: WindowId, buffer: Option<Rc<Buffer>>) {
    if let Some(transaction) = &mut st.shell.transaction {
        transaction.waiting.remove(&w);
    }
    if let Some(buffer) = buffer.filter(|_| st.shell.shown.windows.iter().any(|p| p.window == w)) {
        st.shell.ghosts.insert(w, buffer);
    }
}
