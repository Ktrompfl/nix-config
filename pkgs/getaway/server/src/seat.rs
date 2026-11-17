use crate::{
    ClientId, Key, State, constraints,
    cursor::Cursor,
    dnd,
    privileged::{idle_notify, layer_shell, session_lock, virtual_input},
    scene, selection,
    surface::Role,
    xdg,
};
use backend::{InputEvent, Scroll};
use kbvm::{
    Components, GroupIndex, Keycode, ModifierMask,
    lookup::LookupTable,
    state_machine::{self, Direction, StateMachine},
    xkb::{diagnostic::WriteToLog, rmlvo},
};
use rustix::fs::{MemfdFlags, SealFlags, fcntl_add_seals, memfd_create};
use std::time::{Duration, Instant};
use std::{
    collections::HashSet,
    fs::File,
    os::{
        fd::{AsFd, OwnedFd},
        unix::fs::FileExt,
    },
};
use wire::*;
use wm::{Hit, Rect};

const REPEAT_DELAY: Duration = Duration::from_millis(600);
const REPEAT_INTERVAL: Duration = Duration::from_millis(40);
const ESCAPE: u32 = 0xff1b;
const RECENT_SERIALS: usize = 16;

#[derive(Default)]
pub struct Seat {
    keymap: Option<(OwnedFd, u32)>,
    keymap_text: Option<Vec<u8>>,
    lookup: Option<LookupTable>,
    machine: Option<(StateMachine, state_machine::State, Components)>,
    mods: [u32; 4],
    keyboard_focus: Option<Key>,
    pointer_focus: Option<Key>,
    layer_focus: Option<Key>,
    pointer: Option<(f64, f64)>,
    local: (f64, f64),
    buttons: u32,
    grab: Option<Key>,
    down: HashSet<u32>,
    logical: HashSet<u32>,
    discrete: [i32; 2],
    swallowed: HashSet<u32>,
    virtual_active: Option<Key>,
    enter_serial: u32,
    repeat: Option<(u32, u32, Instant)>,
    hit: Hit,
    inactive: bool,
    serials: Vec<(ClientId, u32)>,
    pub constrained: Option<Key>,
    pub gesture: Option<(backend::GestureKind, Key)>,
}

impl Seat {
    pub fn keyboard_client(&self) -> Option<ClientId> {
        self.keyboard_focus.map(|k| k.0)
    }

    pub fn pointer_client(&self) -> Option<ClientId> {
        self.pointer_focus.map(|k| k.0)
    }

    pub fn keyboard_focus(&self) -> Option<Key> {
        self.keyboard_focus
    }

    pub fn local(&self) -> (f64, f64) {
        self.local
    }

    pub fn enter_serial(&self) -> u32 {
        self.enter_serial
    }

    pub fn pointer(&self) -> Option<(f64, f64)> {
        self.pointer
    }
}

pub fn pointer_target(st: &State) -> Option<Key> {
    let lock = |k: &Key| matches!(st.surfaces.get(k).map(|s| &s.role), Some(Role::LockSurface(..)));
    st.seat.pointer_focus.filter(|k| !session_lock::locked(st) || lock(k))
}

fn input_serial(st: &mut State, c: ClientId) -> u32 {
    let serial = st.next_serial();
    if st.seat.serials.len() >= RECENT_SERIALS {
        st.seat.serials.remove(0);
    }
    st.seat.serials.push((c, serial));
    serial
}

pub fn recent_input(st: &State, c: ClientId, serial: u32) -> bool {
    st.seat.serials.contains(&(c, serial))
}

impl wl_seat::Handler for State {
    fn get_pointer(&mut self, cx: Cx, pointer: Obj) {
        let c = cx.client;
        if let Some(focus) = self.seat.pointer_focus.filter(|f| f.0 == c) {
            let (serial, (sx, sy)) = (self.seat.enter_serial, self.seat.local);
            self.send(c, |out| {
                wl_pointer::enter(out, pointer, serial, focus.1, sx.into(), sy.into());
                wl_pointer::frame(out, pointer);
            });
        }
    }

    fn get_keyboard(&mut self, cx: Cx, keyboard: Obj) {
        let c = cx.client;
        let client = self.client(c);
        client.keymap = Some(client.keymap.flatten());
        send_keymap(self, c, keyboard);
        if let Some(focus) = self.seat.keyboard_focus.filter(|f| f.0 == c) {
            let (serial, mods, keys) = (self.next_serial(), active_mods(self), logical_keys(self));
            self.send(c, |out| enter(out, keyboard, serial, focus.1, &keys, mods));
        }
    }

    fn get_touch(&mut self, _: Cx, _: Obj) {}
}

impl wl_keyboard::Handler for State {}

impl wl_touch::Handler for State {}

fn send_keymap(st: &mut State, c: ClientId, keyboard: Obj) {
    let source = st.clients.get(&c).and_then(|client| virtual_input::keyboard(st, client.keymap.flatten()?));
    let keymap = source
        .map(|v| &v.keymap)
        .or(st.seat.keymap.as_ref())
        .and_then(|(fd, size)| Some((fd.try_clone().ok()?, *size)));
    let Some(client) = st.clients.get_mut(&c) else { return };
    if let Some((fd, size)) = keymap {
        wl_keyboard::keymap(&mut client.out, keyboard, wl_keyboard::keymap_format::XKB_V1, fd.as_fd(), size);
    }
    let rate = (1000 / REPEAT_INTERVAL.as_millis().max(1)) as i32;
    wl_keyboard::repeat_info(&mut client.out, keyboard, rate, REPEAT_DELAY.as_millis() as i32);
}

fn logical_keys(st: &State) -> Vec<u8> {
    st.seat.logical.iter().flat_map(|k| k.to_ne_bytes()).collect()
}

fn enter(out: &mut Out, keyboard: Obj, serial: u32, surface: u32, keys: &[u8], mods: [u32; 4]) {
    let [depressed, latched, locked, group] = mods;
    wl_keyboard::enter(out, keyboard, serial, surface, keys);
    wl_keyboard::modifiers(out, keyboard, serial, depressed, latched, locked, group);
}

fn keyboards(st: &mut State, c: ClientId, f: impl Fn(&mut Out, Obj)) {
    if let Some(client) = st.clients.get_mut(&c) {
        client.each(Interface::WlKeyboard, f);
    }
}

fn pointers(st: &mut State, c: ClientId, f: impl Fn(&mut Out, Obj)) {
    if let Some(client) = st.clients.get_mut(&c) {
        client.each(Interface::WlPointer, |out, pointer| {
            f(out, pointer);
            wl_pointer::frame(out, pointer);
        });
    }
}

#[derive(Clone, Copy)]
pub struct KeymapNames {
    pub layout: &'static str,
    pub variant: &'static str,
    pub options: &'static [&'static str],
}

pub fn input(st: &mut State, event: InputEvent) {
    match event {
        InputEvent::Keymap(fd, size) => keymap(st, fd, size),
        InputEvent::Key { key, pressed } => {
            idle_notify::activity(st);
            if toggle(&mut st.seat.down, key, pressed) {
                deliver_key(st, key, pressed);
                if st.backend.computes_modifiers() {
                    track_modifiers(st, key, pressed);
                }
            }
        }
        InputEvent::Modifiers(mods) => {
            st.seat.mods = mods;
            use_keymap(st, None);
            send_modifiers(st, mods);
        }
        InputEvent::Motion { output, x, y } => {
            if let Some(r) = crate::output::find(st, output).map(|o| o.rect) {
                motion(st, f64::from(r.x) + x, f64::from(r.y) + y);
            }
        }
        InputEvent::Button { button: b, pressed } => button(st, b, pressed),
        InputEvent::Scroll(s) => scroll(st, s),
        InputEvent::PointerLeft => {
            leave_focus(st);
            (st.seat.pointer, st.seat.hit) = (None, Hit::None);
            st.render_dirty = true;
        }
        InputEvent::RelativeMotion { delta, usec } => {
            constraints::relative_motion(st, delta, usec);
            if !st.backend.absolute_pointer() {
                move_pointer(st, delta[0], delta[1]);
            }
        }
        InputEvent::Gesture(gesture) => crate::gestures::event(st, gesture),
    }
}

fn keymap(st: &mut State, fd: OwnedFd, size: u32) {
    let text = read_keymap(&fd, size);
    if text.is_some() && text == st.seat.keymap_text {
        return;
    }
    st.seat.keymap_text = text;
    if let Some(keymap) = parse_keymap(&fd, size) {
        let builder = keymap.to_builder();
        let machine = builder.build_state_machine();
        let state = machine.create_state();
        st.seat.lookup = Some(builder.build_lookup_table());
        st.seat.machine = Some((machine, state, Components::default()));
    }
    st.seat.keymap = Some((fd, size));
    st.seat.virtual_active = None;
    let clients: Vec<ClientId> = st.clients.keys().copied().collect();
    for c in clients {
        st.client(c).keymap = None;
        sync_keymap(st, c);
    }
}

fn sync_keymap(st: &mut State, c: ClientId) {
    let active = st.seat.virtual_active;
    let Some(client) = st.clients.get_mut(&c).filter(|client| client.keymap != Some(active)) else { return };
    client.keymap = Some(active);
    for keyboard in client.of(Interface::WlKeyboard).collect::<Vec<_>>() {
        send_keymap(st, c, keyboard);
    }
}

fn use_keymap(st: &mut State, source: Option<Key>) {
    if st.seat.virtual_active != source {
        st.seat.virtual_active = source;
        if let Some(c) = st.seat.keyboard_client() {
            sync_keymap(st, c);
        }
        send_modifiers(st, active_mods(st));
    }
}

fn active_mods(st: &State) -> [u32; 4] {
    st.seat.virtual_active.and_then(|k| virtual_input::keyboard(st, k)).map_or(st.seat.mods, |v| v.mods)
}

pub fn virtual_keymap_changed(st: &mut State, keyboard: Key) {
    st.clients.values_mut().filter(|c| c.keymap == Some(Some(keyboard))).for_each(|c| c.keymap = None);
    if let Some(c) = st.seat.keyboard_client().filter(|_| st.seat.virtual_active == Some(keyboard)) {
        sync_keymap(st, c);
    }
}

pub fn virtual_keyboard_gone(st: &mut State, keyboard: Key) {
    if st.seat.virtual_active == Some(keyboard) {
        use_keymap(st, None);
    }
}

pub fn virtual_modifiers(st: &mut State, keyboard: Key) {
    match st.seat.virtual_active == Some(keyboard) {
        true => send_modifiers(st, active_mods(st)),
        false => use_keymap(st, Some(keyboard)),
    }
}

pub fn virtual_key(st: &mut State, keyboard: Key, key: u32, pressed: bool) {
    idle_notify::activity(st);
    let Some(v) = virtual_input::keyboard(st, keyboard) else { return };
    let (sym, mods) = (v.lookup.as_ref().and_then(|l| lookup(l, v.mods[3], key)), v.mods[0]);
    if let Some(sym) = sym.filter(|_| pressed)
        && binding(st, mods, sym).is_some()
    {
        st.seat.swallowed.insert(key);
        return;
    }
    if !pressed {
        st.seat.swallowed.remove(&key);
    }
    use_keymap(st, Some(keyboard));
    send_key(st, key, pressed);
}

pub fn initial_keymap(st: &mut State, names: Option<KeymapNames>) -> std::io::Result<()> {
    let text = match names {
        Some(names) => {
            let groups = [rmlvo::Group { layout: names.layout, variant: names.variant }];
            let mut context = kbvm::xkb::Context::builder();
            option_env!("GETAWAY_XKB_ROOT").inspect(|root| context.append_path(root));
            let context = context.build();
            context.keymap_from_names(WriteToLog, None, None, Some(&groups), Some(names.options)).format().to_string()
        }
        None => include_str!("default.xkb").to_string(),
    };
    let text = [text.as_bytes(), &[0]].concat();
    let fd = memfd_create("getaway-keymap", MemfdFlags::CLOEXEC | MemfdFlags::ALLOW_SEALING)?;
    File::from(fd.try_clone()?).write_all_at(&text, 0)?;
    fcntl_add_seals(&fd, SealFlags::WRITE | SealFlags::SHRINK | SealFlags::GROW)?;
    keymap(st, fd, text.len() as u32);
    Ok(())
}

fn read_keymap(fd: &OwnedFd, size: u32) -> Option<Vec<u8>> {
    let mut text = vec![0; size as usize];
    File::from(fd.try_clone().ok()?).read_exact_at(&mut text, 0).ok()?;
    Some(text)
}

fn parse_keymap(fd: &OwnedFd, size: u32) -> Option<kbvm::xkb::Keymap> {
    parse_keymap_text(&read_keymap(fd, size)?)
}

pub fn parse_keymap_text(text: &[u8]) -> Option<kbvm::xkb::Keymap> {
    let text = text.strip_suffix(&[0]).unwrap_or(text);
    kbvm::xkb::Context::default().keymap_from_bytes(WriteToLog, None, text).ok()
}

fn track_modifiers(st: &mut State, key: u32, pressed: bool) {
    let Some((machine, state, components)) = st.seat.machine.as_mut() else { return };
    let mut events = vec![];
    let direction = if pressed { Direction::Down } else { Direction::Up };
    machine.handle_key(state, &mut events, Keycode::from_evdev(key), direction);
    let changed = events.into_iter().fold(false, |changed, e| components.apply_event(e) | changed);
    if changed {
        let c = *components;
        input(st, InputEvent::Modifiers([c.mods_pressed.0, c.mods_latched.0, c.mods_locked.0, c.group.0]));
    }
}

fn keysym(st: &State, key: u32) -> Option<u32> {
    lookup(st.seat.lookup.as_ref()?, st.seat.mods[3], key)
}

fn lookup(table: &LookupTable, group: u32, key: u32) -> Option<u32> {
    let lookup = table.lookup(GroupIndex(group), ModifierMask(0), Keycode::from_evdev(key));
    lookup.into_iter().next().map(|p| p.keysym().0)
}

pub fn session(st: &mut State, active: bool) {
    (st.seat.repeat, st.seat.buttons, st.seat.mods, st.seat.inactive) = (None, 0, [0; 4], !active);
    (st.seat.grab, st.seat.discrete) = (None, [0; 2]);
    st.seat.swallowed.clear();
    st.seat.down.clear();
    if let Some((machine, state, components)) = st.seat.machine.as_mut() {
        (*state, *components) = (machine.create_state(), Components::default());
    }
    match active {
        true => {
            rehover(st);
            (st.render_dirty, st.shell.dirty) = (true, true);
        }
        false => {
            if dnd::active(st) {
                dnd::abort(st);
            }
            set_keyboard_focus(st, None);
            leave_focus(st);
        }
    }
}

fn send_modifiers(st: &mut State, mods: [u32; 4]) {
    let Some(focus) = st.seat.keyboard_focus else { return };
    let serial = st.next_serial();
    let [depressed, latched, locked, group] = mods;
    keyboards(st, focus.0, |out, k| wl_keyboard::modifiers(out, k, serial, depressed, latched, locked, group));
}

fn deliver_key(st: &mut State, key: u32, pressed: bool) {
    if pressed && dnd::active(st) && keysym(st, key) == Some(ESCAPE) {
        st.seat.swallowed.insert(key);
        return dnd::abort(st);
    }
    if let Some(sym) = keysym(st, key).filter(|_| pressed)
        && let Some(repeat) = binding(st, st.seat.mods[0], sym)
    {
        st.seat.swallowed.insert(key);
        st.seat.repeat = repeat.then(|| (key, sym, Instant::now() + REPEAT_DELAY));
        return;
    }
    if st.seat.repeat.is_some_and(|r| r.0 == key) {
        st.seat.repeat = None;
    }
    if !pressed {
        st.seat.swallowed.remove(&key);
    }
    use_keymap(st, None);
    send_key(st, key, pressed);
}

fn toggle(keys: &mut HashSet<u32>, key: u32, pressed: bool) -> bool {
    if pressed { keys.insert(key) } else { keys.remove(&key) }
}

fn binding(st: &mut State, mods: u32, sym: u32) -> Option<bool> {
    let mut cx = wm::Cx::default();
    let repeat = st.policy.key(mods, sym, session_lock::locked(st), &mut cx)?;
    st.shell.dirty = true;
    st.run(cx);
    Some(repeat)
}

fn send_key(st: &mut State, key: u32, pressed: bool) {
    let Some(focus) = st.seat.keyboard_focus else { return };
    if !toggle(&mut st.seat.logical, key, pressed) {
        return;
    }
    let (serial, time) = (input_serial(st, focus.0), st.time());
    let state = if pressed { wl_keyboard::key_state::PRESSED } else { wl_keyboard::key_state::RELEASED };
    keyboards(st, focus.0, |out, k| wl_keyboard::key(out, k, serial, time, key, state));
}

pub fn repeat_deadline(st: &State) -> Option<Instant> {
    st.seat.repeat.map(|r| r.2)
}

pub fn check_repeat(st: &mut State) {
    let Some((key, sym, due)) = st.seat.repeat.filter(|r| r.2 <= Instant::now()) else { return };
    st.seat.repeat = Some((key, sym, due + REPEAT_INTERVAL));
    binding(st, st.seat.mods[0], sym);
}

pub fn update_focus(st: &mut State) {
    if st.seat.inactive {
        return set_keyboard_focus(st, None);
    }
    if session_lock::locked(st) {
        return set_keyboard_focus(st, session_lock::focus(st));
    }
    let layer = |on_demand| layer_shell::keyboard_focus(st, on_demand);
    let exclusive = layer(false);
    let on_demand = st.seat.layer_focus.filter(|k| layer(true) == Some(*k));
    let grabbing = grabbing_popup(st);
    let window = st.shell.target.focus.and_then(|w| st.windows.get(&w).copied());
    set_keyboard_focus(st, exclusive.or(on_demand).or(grabbing).or(window));
}

pub fn grabbing_popup(st: &State) -> Option<Key> {
    let grabs = |k: &Key| matches!(st.surfaces.get(k).map(|s| &s.role), Some(Role::Popup(p)) if p.grab);
    st.popups.iter().rev().copied().find(grabs)
}

fn set_keyboard_focus(st: &mut State, focus: Option<Key>) {
    if st.seat.keyboard_focus == focus {
        return;
    }
    let client_changed = st.seat.keyboard_client() != focus.map(|k| k.0);
    let serial = st.next_serial();
    if let Some(old) = st.seat.keyboard_focus.take() {
        keyboards(st, old.0, |out, k| wl_keyboard::leave(out, k, serial, old.1));
    }
    st.seat.keyboard_focus = focus;
    st.seat.logical = st.seat.down.difference(&st.seat.swallowed).copied().collect();
    if client_changed {
        selection::focus_changed(st);
    }
    if let Some(new) = focus {
        sync_keymap(st, new.0);
        let (mods, keys) = (active_mods(st), logical_keys(st));
        keyboards(st, new.0, |out, k| enter(out, k, serial, new.1, &keys, mods));
    }
    constraints::update(st);
}

pub fn motion(st: &mut State, x: f64, y: f64) {
    idle_notify::activity(st);
    let (x, y) = match constraints::active(st) {
        Some((_, true)) => return,
        Some((surface, false)) => constraints::confine(st, surface, (x, y)),
        None => (x, y),
    };
    hover(st, x, y, true);
}

pub fn rehover(st: &mut State) {
    if let Some((surface, true)) = constraints::active(st)
        && let Some(r) = scene::surface_rect(st, surface).filter(|r| r.w > 0 && r.h > 0)
    {
        let pin = |from: i32, local: f64, to: i32| (f64::from(from) + local).clamp(from.into(), f64::from(to - 1));
        let (sx, sy) = st.seat.local;
        st.seat.pointer = Some((pin(r.x, sx, r.right()), pin(r.y, sy, r.bottom())));
    }
    if let Some((x, y)) = st.seat.pointer {
        hover(st, x, y, false);
    }
}

fn hover(st: &mut State, x: f64, y: f64, moved: bool) {
    st.seat.pointer = Some((x, y));
    if st.seat.inactive {
        return;
    }
    if dnd::active(st) {
        return dnd::motion(st, x, y);
    }
    st.render_dirty |= moved;
    let grab = grabbing_popup(st).map(|k| k.0);
    let held = st.seat.grab.and_then(|k| scene::surface_origin(st, k).map(|(ox, oy)| (k, ox, oy)));
    let hit = match held {
        Some((k, ox, oy)) => Some((k, x - f64::from(ox), y - f64::from(oy))),
        None => scene::surface_at(st, x, y).filter(|h| grab.is_none_or(|c| c == h.0.0)),
    };
    let entered = hit.map(|h| h.0) != st.seat.pointer_focus;
    if entered {
        leave_focus(st);
        st.cursor = Cursor::Named("default");
        if let Some((focus, sx, sy)) = hit {
            let serial = st.next_serial();
            st.seat.enter_serial = serial;
            pointers(st, focus.0, |out, p| wl_pointer::enter(out, p, serial, focus.1, sx.into(), sy.into()));
            st.seat.pointer_focus = Some(focus);
        }
        constraints::update(st);
    }
    let at = match hit {
        Some(h) => xdg::window_of(st, scene::root(st, h.0)).map_or(Hit::None, Hit::Window),
        None => match st.shell.shown.windows.iter().rev().find(|p| p.rect.contains(x, y)) {
            Some(p) => Hit::Window(p.window),
            None => crate::output::at(st, x, y).map_or(Hit::None, |o| Hit::Output(o.id)),
        },
    };
    if std::mem::replace(&mut st.seat.hit, at) != at && moved && !session_lock::locked(st) {
        let mut cx = wm::Cx::default();
        st.shell.dirty |= st.policy.wm().pointer(at, &mut cx);
        st.run(cx);
    }
    let Some((focus, sx, sy)) = hit else { return };
    if !entered && !moved && st.seat.local == (sx, sy) {
        return;
    }
    st.seat.local = (sx, sy);
    if constraints::active(st).is_none() {
        constraints::update(st);
    }
    let (time, warp) = (st.time(), !moved && !entered);
    let locked = constraints::active(st) == Some((focus, true));
    let Some(client) = st.clients.get_mut(&focus.0) else { return };
    client.each(Interface::WlPointer, |out, p| {
        match (warp && p.version >= 11, locked) {
            (true, _) => wl_pointer::warp(out, p, sx.into(), sy.into()),
            (false, false) => wl_pointer::motion(out, p, time, sx.into(), sy.into()),
            (false, true) => return,
        }
        wl_pointer::frame(out, p);
    });
}

pub fn warp_to(st: &mut State, x: f64, y: f64) {
    hover(st, x, y, false);
    st.render_dirty = true;
}

pub fn follow(st: &mut State, r: Rect) {
    let locked = matches!(constraints::active(st), Some((_, true)));
    match st.seat.pointer.is_none_or(|(x, y)| !r.contains(x, y)) && !st.seat.inactive && !dnd::active(st) && !locked {
        true => {
            hover(st, f64::from(r.x + r.w / 2), f64::from(r.y + r.h / 2), false);
            st.render_dirty = true;
        }
        false => rehover(st),
    }
}

pub fn move_pointer(st: &mut State, dx: f64, dy: f64) {
    let start = st.seat.pointer.or_else(|| {
        let r = crate::output::primary(st)?.rect;
        Some((f64::from(r.x + r.w / 2), f64::from(r.y + r.h / 2)))
    });
    let Some((x, y)) = start else { return };
    let (nx, ny) = (x + dx, y + dy);
    let target = match crate::output::at(st, nx, ny) {
        Some(_) => (nx, ny),
        None => match crate::output::at(st, x, y).map(|o| o.rect) {
            Some(r) => {
                (nx.clamp(r.x.into(), f64::from(r.right() - 1)), ny.clamp(r.y.into(), f64::from(r.bottom() - 1)))
            }
            None => return,
        },
    };
    motion(st, target.0, target.1);
}

impl wp_pointer_warp_v1::Handler for State {
    fn warp_pointer(&mut self, cx: Cx, surface: u32, _: u32, x: Fixed, y: Fixed, serial: u32) {
        if self.seat.pointer_client() == Some(cx.client)
            && serial == self.seat.enter_serial
            && !matches!(constraints::active(self), Some((_, true)))
            && let Some(rect) = scene::surface_rect(self, (cx.client, surface))
        {
            let (x, y) = (f64::from(rect.x) + f64::from(x), f64::from(rect.y) + f64::from(y));
            if rect.contains(x, y) {
                warp_to(self, x, y);
            }
        }
    }
}

pub fn holds_button(st: &State, c: ClientId) -> bool {
    st.seat.buttons > 0 && st.seat.pointer_client() == Some(c)
}

pub fn leave_focus(st: &mut State) {
    st.seat.grab = None;
    if let Some(old) = st.seat.pointer_focus.take() {
        let serial = st.next_serial();
        pointers(st, old.0, |out, p| wl_pointer::leave(out, p, serial, old.1));
        constraints::update(st);
    }
}

pub fn button(st: &mut State, button: u32, pressed: bool) {
    idle_notify::activity(st);
    st.seat.buttons = if pressed { st.seat.buttons + 1 } else { st.seat.buttons.saturating_sub(1) };
    if dnd::active(st) {
        if st.seat.buttons == 0 {
            dnd::release(st);
        }
        return;
    }
    if pressed && !session_lock::locked(st) {
        click(st);
    }
    if !session_lock::locked(st) {
        let mut cx = wm::Cx::default();
        let consumed = st.policy.wm().button(st.seat.hit, button, pressed, &mut cx);
        st.shell.dirty = true;
        st.run(cx);
        if consumed && pressed {
            st.seat.swallowed.insert(button);
        }
    }
    deliver_button(st, button, pressed);
    if st.seat.buttons == 0 && st.seat.grab.take().is_some() {
        rehover(st);
    }
}

fn deliver_button(st: &mut State, button: u32, pressed: bool) {
    if !pressed && st.seat.swallowed.remove(&button) || st.seat.swallowed.contains(&button) {
        return;
    }
    let Some(focus) = pointer_target(st) else { return };
    let (serial, time) = (input_serial(st, focus.0), st.time());
    let state = if pressed { wl_pointer::button_state::PRESSED } else { wl_pointer::button_state::RELEASED };
    pointers(st, focus.0, |out, p| wl_pointer::button(out, p, serial, time, button, state));
    if pressed {
        st.seat.grab.get_or_insert(focus);
    }
}

pub fn scroll(st: &mut State, scroll: Scroll) {
    idle_notify::activity(st);
    let Some(focus) = pointer_target(st).filter(|_| !dnd::active(st)) else { return };
    let time = st.time();
    let discrete = [0, 1].map(|i| {
        let (acc, v120) = (&mut st.seat.discrete[i], scroll.axes[i].map_or(0, |a| a.value120));
        if v120 == 0 || acc.signum() == -v120.signum() {
            *acc = 0;
        }
        *acc += v120;
        let clicks = *acc / 120;
        *acc -= clicks * 120;
        clicks
    });
    pointers(st, focus.0, |out, p| {
        let tilt_unknown = scroll.source == Scroll::WHEEL_TILT && p.version < 6;
        wl_pointer::axis_source(out, p, if tilt_unknown { Scroll::WHEEL } else { scroll.source });
        for (axis, a) in (0..).zip(scroll.axes).filter_map(|(i, a)| Some((i, a?))) {
            if a.value == 0.0 && a.value120 == 0 {
                wl_pointer::axis_stop(out, p, time, axis);
                continue;
            }
            wl_pointer::axis_relative_direction(out, p, axis, u32::from(a.inverted));
            match (p.version, discrete[axis as usize]) {
                (8.., _) if a.value120 != 0 => wl_pointer::axis_value120(out, p, axis, a.value120),
                (5..8, clicks) if clicks != 0 => wl_pointer::axis_discrete(out, p, axis, clicks),
                _ => {}
            }
            wl_pointer::axis(out, p, time, axis, a.value.into());
        }
    });
}

fn click(st: &mut State) {
    let target = st.seat.pointer_focus;
    let role = target.and_then(|k| st.surfaces.get(&k)).map(|s| &s.role);
    let (popup, layer) = (matches!(role, Some(Role::Popup(_))), matches!(role, Some(Role::Layer(_))));
    let window = target.and_then(|k| xdg::window_of(st, k));
    if !popup && st.popups.iter().any(|k| Some(k.0) != target.map(|t| t.0)) {
        xdg::dismiss_popups(st);
    }
    if layer || window.is_some() {
        st.seat.layer_focus = target.filter(|_| layer);
    }
}

pub fn surface_gone(st: &mut State, key: Key) {
    if matches!(st.cursor, Cursor::Surface { key: k, .. } if k == key) {
        st.cursor = Cursor::Named("default");
    }
    for focus in [&mut st.seat.keyboard_focus, &mut st.seat.layer_focus] {
        if *focus == Some(key) {
            *focus = None;
        }
    }
    if st.seat.pointer_focus == Some(key) {
        (st.seat.pointer_focus, st.seat.grab) = (None, None);
        rehover(st);
    }
}
