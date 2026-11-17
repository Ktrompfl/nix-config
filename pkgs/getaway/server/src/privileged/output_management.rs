use crate::{
    ClientId, Key, State, caps,
    client::Data,
    output::{self, Override},
};
use backend::{Mode, OutputId, OutputInfo};
use std::collections::HashMap;
use wire::*;

#[derive(Default)]
pub struct Management {
    serial: u32,
    snapshot: Vec<(OutputId, Vec<Mode>, HeadState)>,
}

pub struct Head {
    output: OutputId,
    manager: u32,
    modes: Vec<(Obj, Mode)>,
    sent: Option<HeadState>,
}

#[derive(Clone, Copy, PartialEq)]
struct HeadState {
    enabled: bool,
    mode: Mode,
    position: (i32, i32),
    transform: u32,
    scale: f64,
    vrr: bool,
}

fn head_state(o: &output::Output) -> HeadState {
    let (enabled, mode, transform, scale, vrr) = (o.enabled, o.info.mode, o.transform, o.scale, o.vrr);
    HeadState { enabled, mode, position: (o.rect.x, o.rect.y), transform, scale, vrr }
}

pub struct Configuration {
    serial: u32,
    heads: HashMap<OutputId, Option<Override>>,
    used: bool,
}

pub fn bound(st: &mut State, c: ClientId, manager: Obj) {
    let outputs: Vec<OutputId> = st.outputs.iter().map(|o| o.id).collect();
    outputs.into_iter().for_each(|o| announce(st, c, manager, o));
    let serial = st.management.serial;
    st.send(c, |out| zwlr_output_manager_v1::done(out, manager, serial));
}

fn announce(st: &mut State, c: ClientId, manager: Obj, id: OutputId) {
    let Some(info) = output::find(st, id).map(|o| o.info.clone()) else { return };
    let h = Head { output: id, manager: manager.id, modes: vec![], sent: None };
    let head = st.client(c).add_server(Interface::ZwlrOutputHeadV1, manager.version, Data::Head(h));
    let [name, make, model, serial] = info.identity();
    let known: Vec<&str> = [make, model, serial].into_iter().filter(|s| !s.is_empty()).collect();
    st.send(c, |out| {
        zwlr_output_manager_v1::head(out, manager, head.id);
        zwlr_output_head_v1::name(out, head, name);
        zwlr_output_head_v1::description(out, head, &format!("{} ({name})", known.join(" ")));
        if head.version >= 2 {
            let events = [zwlr_output_head_v1::make, zwlr_output_head_v1::model, zwlr_output_head_v1::serial_number];
            events
                .into_iter()
                .zip([make, model, serial])
                .filter(|e| !e.1.is_empty())
                .for_each(|(e, v)| e(out, head, v));
        }
    });
    send_state(st, c, head);
}

fn head(st: &State, key: Key) -> Option<&Head> {
    match st.data(key)? {
        Data::Head(h) => Some(h),
        _ => None,
    }
}

fn send_state(st: &mut State, c: ClientId, head: Obj) {
    let Some(h) = self::head(st, (c, head.id)) else { return };
    let Some(o) = output::find(st, h.output) else { return };
    let (info, s) = (o.info.clone(), head_state(o));
    let stale: Vec<Obj> = h.modes.iter().filter(|m| !info.modes.contains(&m.1)).map(|m| m.0).collect();
    let new: Vec<Mode> = info.modes.iter().copied().filter(|m| !h.modes.iter().any(|k| k.1 == *m)).collect();
    let new: Vec<(Obj, Mode)> = new
        .into_iter()
        .map(|m| (st.client(c).add_server(Interface::ZwlrOutputModeV1, head.version.min(3), Data::None), m))
        .collect();
    let Some(Data::Head(h)) = st.data_mut((c, head.id)) else { return };
    h.modes.retain(|m| !stale.contains(&m.0));
    h.modes.extend_from_slice(&new);
    let current = h.modes.iter().find(|m| m.1 == info.mode).map(|m| m.0.id);
    let old = h.sent.replace(s).filter(|o| o.enabled == s.enabled);
    st.send(c, |out| {
        stale.iter().for_each(|&m| zwlr_output_mode_v1::finished(out, m));
        for &(obj, mode) in &new {
            zwlr_output_head_v1::mode(out, head, obj.id);
            zwlr_output_mode_v1::size(out, obj, mode.width, mode.height);
            zwlr_output_mode_v1::refresh(out, obj, mode.refresh as i32);
            if info.preferred == Some(mode) {
                zwlr_output_mode_v1::preferred(out, obj);
            }
        }
        if old.is_none() {
            zwlr_output_head_v1::enabled(out, head, i32::from(s.enabled));
        }
        if s.enabled {
            if old.is_none_or(|o| o.mode != s.mode) {
                current.into_iter().for_each(|m| zwlr_output_head_v1::current_mode(out, head, m));
            }
            if old.is_none_or(|o| o.position != s.position) {
                zwlr_output_head_v1::position(out, head, s.position.0, s.position.1);
            }
            if old.is_none_or(|o| o.transform != s.transform) {
                zwlr_output_head_v1::transform(out, head, s.transform as i32);
            }
            if old.is_none_or(|o| o.scale != s.scale) {
                zwlr_output_head_v1::scale(out, head, s.scale.into());
            }
        }
        if old.is_none_or(|o| o.vrr != s.vrr) {
            zwlr_output_head_v1::adaptive_sync(out, head, u32::from(s.vrr));
        }
    });
}

pub fn changed(st: &mut State) {
    let snapshot: Vec<_> = st.outputs.iter().map(|o| (o.id, o.info.modes.clone(), head_state(o))).collect();
    if snapshot == st.management.snapshot {
        return;
    }
    let old = std::mem::replace(&mut st.management.snapshot, snapshot);
    let new: Vec<OutputId> = st.outputs.iter().map(|o| o.id).filter(|&o| !old.iter().any(|s| s.0 == o)).collect();
    st.management.serial += 1;
    let serial = st.management.serial;
    let managers: Vec<(ClientId, Obj)> = st
        .objects(caps::OUTPUT_MANAGEMENT)
        .filter(|o| o.1.interface == Interface::ZwlrOutputManagerV1)
        .map(|(c, o)| (c, o.obj))
        .collect();
    for (c, manager) in managers {
        let heads = st.clients[&c].objects().filter_map(|o| match &o.state {
            Data::Head(h) if h.manager == manager.id => Some((o.obj, h.output)),
            _ => None,
        });
        for (head, id) in heads.collect::<Vec<_>>() {
            match output::find(st, id) {
                Some(_) => send_state(st, c, head),
                None => finish_head(st, c, head),
            }
        }
        new.iter().for_each(|&o| announce(st, c, manager, o));
        st.send(c, |out| zwlr_output_manager_v1::done(out, manager, serial));
    }
}

fn finish_head(st: &mut State, c: ClientId, head: Obj) {
    let Some(Data::Head(Head { modes, .. })) = st.data_mut((c, head.id)).map(std::mem::take) else { return };
    st.send(c, |out| {
        modes.iter().for_each(|m| zwlr_output_mode_v1::finished(out, m.0));
        zwlr_output_head_v1::finished(out, head);
    });
}

impl zwlr_output_manager_v1::Handler for State {
    fn create_configuration(&mut self, cx: Cx, id: Obj, serial: u32) {
        let config = Configuration { serial, heads: HashMap::new(), used: false };
        self.set((cx.client, id.id), Data::Config(config));
    }

    fn stop(&mut self, cx: Cx) {
        crate::client::send_destructor(self, cx.client, cx.obj, zwlr_output_manager_v1::finished);
    }
}

impl zwlr_output_head_v1::Handler for State {}

impl zwlr_output_mode_v1::Handler for State {}

fn output_of(st: &State, cx: Cx, head: u32) -> Option<OutputId> {
    self::head(st, (cx.client, head)).map(|h| h.output)
}

fn config(st: &mut State, key: Key) -> Option<&mut Configuration> {
    match st.data_mut(key)? {
        Data::Config(c) => Some(c),
        _ => None,
    }
}

fn unused(st: &mut State, cx: Cx) -> Res {
    match config(st, cx.key()).is_none_or(|c| c.used) {
        true => Err(cx.error(zwlr_output_configuration_v1::error::ALREADY_USED, "configuration already used")),
        false => Ok(()),
    }
}

impl zwlr_output_configuration_v1::Handler for State {
    fn enable_head(&mut self, cx: Cx, id: Obj, head: u32) -> Res {
        let Some(output) = output_of(self, cx, head) else { return Ok(()) };
        configure(self, cx.key(), output, Some(Override::default()))?;
        self.set((cx.client, id.id), Data::ConfigHead(cx.obj.id, output));
        Ok(())
    }

    fn disable_head(&mut self, cx: Cx, head: u32) -> Res {
        let Some(output) = output_of(self, cx, head) else { return Ok(()) };
        configure(self, cx.key(), output, None)
    }

    fn apply(&mut self, cx: Cx) -> Res {
        unused(self, cx)?;
        finish(self, cx, true)
    }

    fn test(&mut self, cx: Cx) -> Res {
        unused(self, cx)?;
        finish(self, cx, false)
    }
}

pub fn config_gone(st: &mut State, key: Key) {
    if let Some(client) = st.clients.get_mut(&key.0) {
        for o in client.objects_mut().filter(|o| matches!(o.state, Data::ConfigHead(config, _) if config == key.1)) {
            o.state = Data::None;
        }
    }
}

fn configure(st: &mut State, key: Key, output: OutputId, changes: Option<Override>) -> Res {
    let Some(config) = config(st, key) else { return Ok(()) };
    if config.heads.insert(output, changes).is_some() {
        let code = zwlr_output_configuration_v1::error::ALREADY_CONFIGURED_HEAD;
        return Err(Error::new(key.1, code, "head configured twice"));
    }
    Ok(())
}

fn config_head(st: &mut State, cx: Cx) -> Option<(&mut Override, Vec<(Obj, Mode)>)> {
    let &mut Data::ConfigHead(config, output) = st.data_mut(cx.key())? else { return None };
    let heads = st.clients.get(&cx.client)?.objects().filter_map(|o| match &o.state {
        Data::Head(h) if h.output == output => Some(h.modes.clone()),
        _ => None,
    });
    let modes = heads.flatten().collect();
    let ch = self::config(st, (cx.client, config))?.heads.get_mut(&output)?.as_mut()?;
    Some((ch, modes))
}

fn once(cx: Cx, twice: bool) -> Res {
    match twice {
        true => Err(cx.error(zwlr_output_configuration_head_v1::error::ALREADY_SET, "property set twice")),
        false => Ok(()),
    }
}

impl zwlr_output_configuration_head_v1::Handler for State {
    fn set_mode(&mut self, cx: Cx, mode: u32) -> Res {
        let Some((ch, modes)) = config_head(self, cx) else { return Ok(()) };
        let Some(&(_, m)) = modes.iter().find(|m| m.0.id == mode) else {
            return Err(cx.error(zwlr_output_configuration_head_v1::error::INVALID_MODE, "unknown mode"));
        };
        once(cx, ch.mode.replace(m).is_some())
    }

    fn set_custom_mode(&mut self, cx: Cx, width: i32, height: i32, refresh: i32) -> Res {
        let Some((ch, _)) = config_head(self, cx) else { return Ok(()) };
        if width <= 0 || height <= 0 || refresh < 0 {
            return Err(cx.error(zwlr_output_configuration_head_v1::error::INVALID_CUSTOM_MODE, "invalid custom mode"));
        }
        once(cx, ch.mode.replace(Mode { width, height, refresh: refresh as u32 }).is_some())
    }

    fn set_position(&mut self, cx: Cx, x: i32, y: i32) -> Res {
        let Some((ch, _)) = config_head(self, cx) else { return Ok(()) };
        once(cx, ch.position.replace((x, y)).is_some())
    }

    fn set_transform(&mut self, cx: Cx, transform: i32) -> Res {
        let Some((ch, _)) = config_head(self, cx) else { return Ok(()) };
        if !(0..8).contains(&transform) {
            return Err(cx.error(zwlr_output_configuration_head_v1::error::INVALID_TRANSFORM, "invalid transform"));
        }
        once(cx, ch.transform.replace(transform as u32).is_some())
    }

    fn set_scale(&mut self, cx: Cx, scale: Fixed) -> Res {
        let Some((ch, _)) = config_head(self, cx) else { return Ok(()) };
        match (f64::from(scale) * 120.0).round() / 120.0 {
            scale if scale <= 0.0 => {
                Err(cx.error(zwlr_output_configuration_head_v1::error::INVALID_SCALE, "scale must be positive"))
            }
            scale => once(cx, ch.scale.replace(scale).is_some()),
        }
    }

    fn set_adaptive_sync(&mut self, cx: Cx, state: u32) -> Res {
        let Some((ch, _)) = config_head(self, cx) else { return Ok(()) };
        if state > 1 {
            let code = zwlr_output_configuration_head_v1::error::INVALID_ADAPTIVE_SYNC_STATE;
            return Err(cx.error(code, "invalid adaptive sync state"));
        }
        once(cx, ch.vrr.replace(state == 1).is_some())
    }
}

fn finish(st: &mut State, cx: Cx, apply: bool) -> Res {
    let (key, obj) = (cx.key(), cx.obj);
    let Some(config) = config(st, key) else { return Ok(()) };
    config.used = true;
    let (serial, heads) = (config.serial, config.heads.clone());
    if serial != st.management.serial {
        st.send(key.0, |out| zwlr_output_configuration_v1::cancelled(out, obj));
        return Ok(());
    }
    if st.outputs.iter().any(|o| !heads.contains_key(&o.id)) {
        let code = zwlr_output_configuration_v1::error::UNCONFIGURED_HEAD;
        return Err(Error::new(obj.id, code, "every head must be enabled or disabled"));
    }
    let Some(overrides) = validate(st, &heads) else {
        st.send(key.0, |out| zwlr_output_configuration_v1::failed(out, obj));
        return Ok(());
    };
    if apply {
        let previous = st.output_overrides.clone();
        st.output_overrides.extend(overrides);
        if !output::reconfigure(st) {
            st.output_overrides = previous;
            output::reconfigure(st);
            st.send(key.0, |out| zwlr_output_configuration_v1::failed(out, obj));
            return Ok(());
        }
    }
    st.send(key.0, |out| zwlr_output_configuration_v1::succeeded(out, obj));
    Ok(())
}

fn validate(st: &State, heads: &HashMap<OutputId, Option<Override>>) -> Option<Vec<([String; 4], Override)>> {
    let infos: Vec<OutputInfo> = st.outputs.iter().map(|o| o.info.clone()).collect();
    let mut overrides = vec![];
    for o in &st.outputs {
        let mut changes = heads[&o.id].unwrap_or_default();
        changes.enabled = Some(heads[&o.id].is_some());
        if let Some(m) = changes.mode {
            changes.mode = Some(*o.info.modes.iter().find(|x| close(x, &m))?);
        }
        if changes.transform.is_some_and(|t| t > 3) {
            return None;
        }
        let identity = output::identity(&o.info);
        let merged = st.output_overrides.get(&identity).copied().unwrap_or_default().merge(changes);
        let mut config = (st.output_config)(&o.info, &infos);
        merged.apply(&mut config);
        if config.enabled
            && let Err(e) = st.backend.test(o.id, config.wanted_mode(), config.vrr)
        {
            log::warn!("output configuration for {} refused: {e}", o.info.name);
            return None;
        }
        overrides.push((identity, merged));
    }
    Some(overrides)
}

fn close(a: &Mode, b: &Mode) -> bool {
    (a.width, a.height) == (b.width, b.height) && (b.refresh == 0 || a.refresh.abs_diff(b.refresh) < 500)
}
