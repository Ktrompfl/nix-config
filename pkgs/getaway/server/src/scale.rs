use crate::{
    Key, State,
    client::{self, Data},
    output,
    privileged::{foreign_toplevel, workspaces},
    scene, surface,
};
use backend::{Mode, OutputId};
use std::collections::HashMap;
use wire::*;
use wm::Rect;

impl wp_viewporter::Handler for State {
    fn get_viewport(&mut self, cx: Cx, id: Obj, surface: u32) -> Res {
        surface::add_extension(self, cx, id, surface, wp_viewporter::error::VIEWPORT_EXISTS).map(drop)
    }
}

impl wp_viewport::Handler for State {
    fn destroy(&mut self, cx: Cx) -> Res {
        if let Some(s) = surface::extended(self, cx, None)? {
            (s.pending.state.source, s.pending.state.destination) = (Some(None), Some(None));
        }
        Ok(())
    }

    fn set_source(&mut self, cx: Cx, x: Fixed, y: Fixed, width: Fixed, height: Fixed) -> Res {
        let Some(s) = surface::extended(self, cx, Some(wp_viewport::error::NO_SURFACE))? else { return Ok(()) };
        let unset = Fixed(-256);
        s.pending.state.source = match [x, y, width, height] {
            all if all == [unset; 4] => Some(None),
            _ if x.0 < 0 || y.0 < 0 || width.0 <= 0 || height.0 <= 0 => {
                return Err(cx.error(wp_viewport::error::BAD_VALUE, "invalid source rectangle"));
            }
            _ => Some(Some([x.into(), y.into(), width.into(), height.into()])),
        };
        Ok(())
    }

    fn set_destination(&mut self, cx: Cx, width: i32, height: i32) -> Res {
        let Some(s) = surface::extended(self, cx, Some(wp_viewport::error::NO_SURFACE))? else { return Ok(()) };
        s.pending.state.destination = match (width, height) {
            (-1, -1) => Some(None),
            _ if width <= 0 || height <= 0 => {
                return Err(cx.error(wp_viewport::error::BAD_VALUE, "invalid destination size"));
            }
            size => Some(Some(size)),
        };
        Ok(())
    }
}

impl wp_fractional_scale_manager_v1::Handler for State {
    fn get_fractional_scale(&mut self, cx: Cx, id: Obj, surface: u32) -> Res {
        let code = wp_fractional_scale_manager_v1::error::FRACTIONAL_SCALE_EXISTS;
        surface::add_extension(self, cx, id, surface, code)?.fractional = Some((id.id, 0));
        update_surfaces(self, false);
        Ok(())
    }
}

impl wp_fractional_scale_v1::Handler for State {}

impl zxdg_output_manager_v1::Handler for State {
    fn get_xdg_output(&mut self, cx: Cx, xdg_output: Obj, output: u32) {
        self.set((cx.client, xdg_output.id), Data::XdgOutput(output, None));
        resync(self);
    }
}

impl zxdg_output_v1::Handler for State {}

impl wl_output::Handler for State {}

pub type Shown = (Rect, Mode, bool, i32, u32, String);

pub fn resync(st: &mut State) {
    st.shell.resync = true;
    sync(st);
}

pub fn sync(st: &mut State) {
    let resync = std::mem::take(&mut st.shell.resync);
    if resync {
        sync_outputs(st);
        workspaces::sync(st);
        foreign_toplevel::sync(st);
    }
    update_surfaces(st, resync);
}

fn sync_outputs(st: &mut State) {
    for client in st.clients.values_mut() {
        let (wl_outputs, mut done) = (client.outputs(), vec![]);
        let find = |id: Option<OutputId>| st.outputs.iter().find(|o| Some(o.id) == id);
        client.each_mut(|out, o| match &mut o.state {
            Data::Output(id, sent) => {
                let Some(x) = find(*id) else { return };
                let (mode, preferred) = (x.info.mode, x.info.preferred == Some(x.info.mode));
                let shown = (x.rect, mode, preferred, x.scale.ceil() as i32, x.transform, x.info.model.clone());
                if sent.as_ref() == Some(&shown) {
                    return;
                }
                let (rect, model) = (x.rect, &x.info.model);
                wl_output::geometry(out, o.obj, rect.x, rect.y, 0, 0, 0, "getaway", model, x.transform as i32);
                let flags = wl_output::mode::CURRENT | if preferred { wl_output::mode::PREFERRED } else { 0 };
                wl_output::mode(out, o.obj, flags, mode.width, mode.height, mode.refresh as i32);
                wl_output::scale(out, o.obj, shown.3);
                if sent.is_none() {
                    wl_output::name(out, o.obj, &x.info.name);
                }
                if sent.as_ref().is_none_or(|s| s.5 != *model) {
                    wl_output::description(out, o.obj, model);
                }
                *sent = Some(shown);
                done.push(o.obj);
            }
            Data::XdgOutput(wl_output, sent) => {
                let Some(&(wl_output, id)) = wl_outputs.iter().find(|w| w.0.id == *wl_output) else { return };
                let Some(x) = find(id).filter(|x| *sent != Some(x.rect)) else { return };
                zxdg_output_v1::logical_position(out, o.obj, x.rect.x, x.rect.y);
                zxdg_output_v1::logical_size(out, o.obj, x.rect.w, x.rect.h);
                if sent.replace(x.rect).is_none() {
                    zxdg_output_v1::name(out, o.obj, &x.info.name);
                }
                match o.obj.version {
                    ..3 => zxdg_output_v1::done(out, o.obj),
                    _ => done.push(wl_output),
                }
            }
            _ => {}
        });
        done.sort_by_key(|o| o.id);
        done.dedup_by_key(|o| o.id);
        done.into_iter().for_each(|obj| wl_output::done(&mut client.out, obj));
    }
}

fn update_surfaces(st: &mut State, resync: bool) {
    let default = output::default_scale(st);
    let placed: Vec<(Key, Vec<OutputId>, f64)> = st
        .surfaces
        .keys()
        .map(|&key| {
            let rect = scene::visible_rect(st, key);
            let on = |o: &&output::Output| o.in_layout() && rect.is_some_and(|r| !o.rect.intersect(r).is_empty());
            let outputs: Vec<&output::Output> = st.outputs.iter().filter(on).collect();
            (
                key,
                outputs.iter().map(|o| o.id).collect(),
                outputs.iter().map(|o| o.scale).reduce(f64::max).unwrap_or(default),
            )
        })
        .collect();
    let mut bound = HashMap::new();
    for (key, outputs, preferred) in placed {
        let (Some(s), Some(client)) = (st.surfaces.get_mut(&key), st.clients.get_mut(&key.0)) else { continue };
        let obj = Obj { id: key.1, version: s.version };
        if resync || s.outputs != outputs {
            let bound = bound.entry(key.0).or_insert_with(|| client.outputs());
            client::enter(&mut s.entered, bound, &outputs, |id, enter| match enter {
                true => wl_surface::enter(&mut client.out, obj, id),
                false => wl_surface::leave(&mut client.out, obj, id),
            });
            s.outputs = outputs;
        }
        let scale = preferred.ceil() as i32;
        if std::mem::replace(&mut s.buffer_scale_sent, scale) != scale {
            wl_surface::preferred_buffer_scale(&mut client.out, obj, scale);
        }
        s.preferred_scale = preferred;
        let fraction = (preferred * 120.0).round() as u32;
        if let Some((id, sent)) = s.fractional.as_mut().filter(|f| f.1 != fraction) {
            *sent = fraction;
            wp_fractional_scale_v1::preferred_scale(&mut client.out, Obj { id: *id, version: 1 }, fraction);
        }
    }
}
