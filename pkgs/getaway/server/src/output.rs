use crate::{
    State,
    client::Data,
    privileged::{layer_shell, session_lock},
    scale,
};
use backend::{Mode, OutputEvent, OutputId, OutputInfo};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    time::{Duration, Instant},
};
use wire::*;
use wm::Rect;

pub const GLOBAL_BASE: u32 = 1000;
pub const VERSION: u32 = 4;

#[derive(Clone, Debug, Default)]
pub struct OutputConfig {
    pub enabled: bool,
    pub mode: Option<(i32, i32, f64)>,
    pub position: Option<(i32, i32)>,
    pub scale: Option<f64>,
    pub transform: u32,
    pub vrr: bool,
    pub workspaces: Vec<usize>,
    pub mirror: Option<String>,
}

impl OutputConfig {
    pub fn wanted_mode(&self) -> Option<Mode> {
        self.mode.map(|(width, height, hz)| Mode { width, height, refresh: (hz * 1000.0).round() as u32 })
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Override {
    pub enabled: Option<bool>,
    pub mode: Option<Mode>,
    pub position: Option<(i32, i32)>,
    pub transform: Option<u32>,
    pub scale: Option<f64>,
    pub vrr: Option<bool>,
}

impl Override {
    pub fn merge(self, newer: Override) -> Override {
        Override {
            enabled: newer.enabled.or(self.enabled),
            mode: newer.mode.or(self.mode),
            position: newer.position.or(self.position),
            transform: newer.transform.or(self.transform),
            scale: newer.scale.or(self.scale),
            vrr: newer.vrr.or(self.vrr),
        }
    }

    pub fn apply(&self, config: &mut OutputConfig) {
        config.enabled = self.enabled.unwrap_or(config.enabled);
        config.mode = self.mode.map(|m| (m.width, m.height, f64::from(m.refresh) / 1000.0)).or(config.mode);
        config.position = self.position.or(config.position);
        config.transform = self.transform.unwrap_or(config.transform);
        config.scale = self.scale.or(config.scale);
        config.vrr = self.vrr.unwrap_or(config.vrr);
    }
}

pub fn identity(info: &OutputInfo) -> [String; 4] {
    info.identity().map(String::from)
}

#[derive(Default)]
pub struct Output {
    pub id: OutputId,
    pub info: OutputInfo,
    pub rect: Rect,
    pub usable: Rect,
    pub scale: f64,
    pub transform: u32,
    pub enabled: bool,
    pub off: bool,
    pub dirty: bool,
    pub vrr: bool,
    pub frames: u64,
    pub damage: Damage,
    pub next_vblank: Option<Instant>,
    pub cursor_shown: Option<(u64, (i32, i32))>,
    pub mirror: Option<OutputId>,
    pub frozen: Option<(gfx::TexId, (i32, i32), u64)>,
    pub render_cost: Duration,
    pub gpu_cost: Duration,
    pub gpu_frame: Option<(u64, Instant)>,
    pub retry: Option<Instant>,
    workspaces: Vec<usize>,
    advertised: bool,
}

impl Output {
    pub fn frame(&self) -> gfx::Frame {
        gfx::Frame { size: (self.info.mode.width, self.info.mode.height), transform: self.transform }
    }

    pub fn in_layout(&self) -> bool {
        self.enabled && self.mirror.is_none()
    }

    pub fn period(&self) -> Duration {
        Duration::from_nanos(1_000_000_000_000 / u64::from(self.info.mode.refresh.max(1000)))
    }

    pub fn render_at(&self, tearing: bool) -> Option<Instant> {
        let budget = (self.render_cost * 2 + Duration::from_micros(1500)).min(self.period());
        let latch = self.next_vblank.filter(|_| !self.vrr && !tearing).map(|vblank| vblank - budget);
        latch.max(self.retry)
    }
}

pub fn event(st: &mut State, event: OutputEvent) {
    if let OutputEvent::Changed(_) | OutputEvent::Removed(_) = event {
        st.scanout_targets.drain().for_each(|(_, texture)| st.renderer.free(texture));
        st.outputs.iter_mut().for_each(|o| o.damage = Damage::default());
    }
    match event {
        OutputEvent::Added(info) => st.outputs.push(Output {
            id: info.id,
            scale: info.scale,
            info,
            dirty: true,
            render_cost: Duration::from_millis(1),
            ..Default::default()
        }),
        OutputEvent::Changed(info) => {
            if let Some(output) = find_mut(st, info.id) {
                output.info = info;
            }
        }
        OutputEvent::Removed(id) => removed(st, id),
        OutputEvent::GammaFailed(id) => return crate::privileged::gamma::failed(st, id),
    }
    reconfigure(st);
}

fn removed(st: &mut State, id: OutputId) {
    if let Some(index) = st.outputs.iter().position(|o| o.id == id) {
        st.outputs[index].enabled = false;
        advertise(st);
        if let Some((texture, ..)) = st.outputs.remove(index).frozen {
            st.renderer.free(texture);
        }
        crate::privileged::output_removed(st, id);
    }
    let time = st.time();
    for ((c, callback), _) in st.presented.extract_if(.., |p| p.1 == id).collect::<Vec<_>>() {
        crate::base::callback_done(st, c, callback, time);
    }
    for ((c, feedback), _) in st.presented_feedback.extract_if(.., |p| p.1 == id).collect::<Vec<_>>() {
        crate::surface::discard(st, c, vec![feedback]);
    }
}

fn effective_config(st: &State, index: usize, infos: &[OutputInfo]) -> OutputConfig {
    let info = &st.outputs[index].info;
    let mut config = (st.output_config)(info, infos);
    if let Some(o) = st.output_overrides.get(&identity(info)) {
        o.apply(&mut config);
    }
    config
}

pub fn reconfigure(st: &mut State) -> bool {
    let mut configured = true;
    let infos: Vec<OutputInfo> = st.outputs.iter().map(|o| o.info.clone()).collect();
    let mut placed_right = 0;
    let configs: Vec<OutputConfig> = (0..st.outputs.len()).map(|i| effective_config(st, i, &infos)).collect();
    let source = |name: &String| {
        let index = st.outputs.iter().position(|o| &o.info.name == name)?;
        let source = &configs[index];
        (source.enabled && source.mirror.is_none()).then_some(st.outputs[index].id)
    };
    let mirrors: Vec<Option<OutputId>> = configs.iter().map(|c| c.mirror.as_ref().and_then(source)).collect();
    for (index, (config, mirror)) in configs.into_iter().zip(mirrors).enumerate() {
        if let Err(e) = st.backend.configure(
            st.outputs[index].id,
            config.wanted_mode(),
            config.vrr,
            config.enabled && !st.outputs[index].off,
        ) {
            log::warn!("cannot configure output {}: {e}", st.outputs[index].info.name);
            configured = false;
        }
        let o = &mut st.outputs[index];
        o.scale = config.scale.unwrap_or(o.info.scale);
        (o.transform, o.enabled, o.vrr, o.workspaces, o.mirror) =
            (config.transform & 3, config.enabled, config.vrr, config.workspaces, mirror);
        let (w, h) = (o.info.mode.width, o.info.mode.height);
        let (w, h) = if o.transform % 2 == 1 { (h, w) } else { (w, h) };
        let size = (logical(w, o.scale), logical(h, o.scale));
        let (x, y) = config.position.unwrap_or((placed_right, 0));
        if o.in_layout() {
            placed_right = placed_right.max(x + size.0);
        }
        o.rect = if o.mirror.is_some() { Rect::default() } else { Rect::new(x, y, size.0, size.1) };
        o.dirty = true;
        o.damage = Damage::default();
    }
    for o in &mut st.outputs {
        if let Some((texture, ..)) = o.frozen.take_if(|_| o.mirror.is_none()) {
            st.renderer.free(texture);
        }
    }
    advertise(st);
    layer_shell::arrange(st);
    session_lock::reconfigure(st);
    crate::privileged::capture::outputs_changed(st);
    crate::privileged::output_management::changed(st);
    notify_wm(st);
    st.shell.resync = true;
    st.render_dirty = true;
    configured
}

#[derive(Default)]
pub struct Damage {
    shown: Vec<(Rect, u64)>,
    history: VecDeque<Rect>,
    frame: u64,
    drawn: HashMap<u64, u64>,
}

impl Damage {
    pub fn update(&mut self, contents: &[(Rect, u64)]) -> Rect {
        if self.shown == contents {
            return Rect::default();
        }
        let old: HashSet<(Rect, u64)> = self.shown.iter().copied().collect();
        let new: HashSet<(Rect, u64)> = contents.iter().copied().collect();
        let changed = old.symmetric_difference(&new).fold(Rect::default(), |d, c| d.union(c.0));
        let (kept, still) =
            (self.shown.iter().filter(|c| new.contains(c)), contents.iter().filter(|c| old.contains(c)));
        let changed = kept.zip(still).filter(|(a, b)| a != b).fold(changed, |d, (a, b)| d.union(a.0).union(b.0));
        self.shown.clear();
        self.shown.extend_from_slice(contents);
        self.frame += 1;
        self.history.push_front(changed);
        self.history.truncate(4);
        changed
    }

    pub fn region(&self, target: u64) -> Option<Rect> {
        let age = (self.frame - self.drawn.get(&target)?) as usize;
        (age <= self.history.len()).then(|| self.history.iter().take(age).fold(Rect::default(), |d, r| d.union(*r)))
    }

    pub fn drawn(&mut self, target: u64) {
        self.drawn.insert(target, self.frame);
    }

    pub fn forget(&mut self, live: &[u64]) {
        self.drawn.retain(|target, _| live.contains(target));
    }
}

pub fn notify_wm(st: &mut State) {
    let outputs: Vec<wm::Output> = st
        .outputs
        .iter()
        .filter(|o| o.in_layout())
        .map(|o| wm::Output { id: o.id, full: o.rect, usable: o.usable, workspaces: o.workspaces.clone() })
        .collect();
    st.policy.wm().outputs_changed(&outputs);
    st.shell.dirty = true;
}

fn advertise(st: &mut State) {
    let changed: Vec<(OutputId, bool)> = st
        .outputs
        .iter_mut()
        .filter(|o| o.in_layout() != o.advertised)
        .map(|o| {
            o.advertised = o.in_layout();
            (o.id, o.advertised)
        })
        .collect();
    for client in st.clients.values_mut() {
        for o in client.objects_mut() {
            if let Data::Output(Some(x), _) = o.state
                && changed.contains(&(x, false))
            {
                o.state = Data::Output(None, None);
            }
        }
    }
    match changed.iter().any(|c| !c.1) {
        true => scale::resync(st),
        false => st.shell.resync = true,
    }
    for (id, advertised) in changed {
        for client in st.clients.values_mut() {
            client.each(Interface::WlRegistry, |out, registry| match advertised {
                true => wl_registry::global(out, registry, GLOBAL_BASE + id, wl_output::NAME, VERSION),
                false => wl_registry::global_remove(out, registry, GLOBAL_BASE + id),
            });
        }
    }
}

pub fn advertised(st: &State) -> Vec<u32> {
    st.outputs.iter().filter(|o| o.advertised).map(|o| GLOBAL_BASE + o.id).collect()
}

pub fn bind(st: &mut State, c: crate::ClientId, name: u32, id: u32, version: u32) -> Res {
    if !(1..=VERSION).contains(&version) {
        let message = format!("cannot bind {} version {version}", wl_output::NAME);
        return Err(Error::new(DISPLAY.id, wl_display::error::INVALID_OBJECT, message));
    }
    let output = name - GLOBAL_BASE;
    st.client(c).add(id, Interface::WlOutput, version, 0)?;
    if find(st, output).is_some_and(|o| o.advertised) {
        st.set((c, id), Data::Output(Some(output), None));
        scale::resync(st);
    }
    Ok(())
}

pub fn find(st: &State, id: OutputId) -> Option<&Output> {
    st.outputs.iter().find(|o| o.id == id)
}

pub fn find_mut(st: &mut State, id: OutputId) -> Option<&mut Output> {
    st.outputs.iter_mut().find(|o| o.id == id)
}

pub fn at(st: &State, x: f64, y: f64) -> Option<&Output> {
    st.outputs.iter().filter(|o| o.in_layout()).find(|o| o.rect.contains(x, y))
}

pub fn in_layout(st: &State) -> Vec<OutputId> {
    st.outputs.iter().filter(|o| o.in_layout()).map(|o| o.id).collect()
}

pub fn primary(st: &State) -> Option<&Output> {
    st.outputs.iter().find(|o| o.in_layout())
}

pub fn focused(st: &State) -> Option<OutputId> {
    let pointer = st.seat.pointer().and_then(|(x, y)| at(st, x, y));
    pointer.or_else(|| primary(st)).map(|o| o.id)
}

pub fn containing(st: &State, rect: Rect) -> Option<&Output> {
    let (x, y) = (f64::from(rect.x), f64::from(rect.y));
    at(st, x, y).or_else(|| primary(st))
}

pub fn bounds(st: &State) -> Option<Rect> {
    let rects = st.outputs.iter().filter(|o| o.in_layout()).map(|o| o.rect);
    rects.reduce(|a, b| {
        let (x, y) = (a.x.min(b.x), a.y.min(b.y));
        Rect::new(x, y, a.right().max(b.right()) - x, a.bottom().max(b.bottom()) - y)
    })
}

fn logical(physical: i32, scale: f64) -> i32 {
    let covers = |l: i32| (f64::from(l) * scale).round() as i32 >= physical;
    let mut l = (f64::from(physical) / scale).ceil() as i32;
    while l > 1 && covers(l - 1) {
        l -= 1;
    }
    l
}

pub fn default_scale(st: &State) -> f64 {
    primary(st).map_or(1.0, |o| o.scale)
}
