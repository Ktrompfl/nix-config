use crate::{
    Key, State, caps,
    client::{self, Data},
    xdg,
};
use backend::OutputId;
use wire::*;
use wm::{Cause, WindowId};

type Status = (bool, bool, Option<u64>);

#[derive(Default)]
pub struct Handle {
    list: u32,
    mapping: Option<u64>,
    shown: Option<(String, String, Status)>,
    entered: Vec<u32>,
}

pub fn sync(st: &mut State) {
    let target = &st.shell.target;
    let windows: Vec<(u64, String, String, Status, Vec<OutputId>)> = xdg::managed_windows(st)
        .map(|(w, mapping, info)| {
            let placed = target.windows.iter().find(|p| p.window == w);
            let on = |o: &&crate::output::Output| placed.is_some_and(|p| !o.rect.intersect(p.rect).is_empty());
            let outputs = st.outputs.iter().filter(|o| o.in_layout()).filter(on).map(|o| o.id).collect();
            let parent = info.parent.and_then(|p| xdg::managed_windows(st).find(|w| w.0 == p)).map(|w| w.1);
            let state = (target.focus == Some(w), placed.is_some_and(|p| p.fullscreen), parent);
            (mapping, info.title.clone(), info.app_id.clone(), state, outputs)
        })
        .collect();
    for client in st.clients.values_mut().filter(|c| c.caps & caps::FOREIGN_TOPLEVEL != 0) {
        let mut new = vec![];
        client.each_mut(|_, o| {
            if let Data::Toplevels(announced) = &mut o.state {
                new.extend(windows.iter().filter(|w| !announced.contains(&w.0)).map(|w| (o.obj, o.interface, w.0)));
                *announced = windows.iter().map(|w| w.0).collect();
            }
        });
        for (list, interface, mapping) in new {
            let h = Data::Handle(Handle { list: list.id, mapping: Some(mapping), ..Default::default() });
            if interface == Interface::ZwlrForeignToplevelManagerV1 {
                let handle = client.add_server(Interface::ZwlrForeignToplevelHandleV1, list.version, h);
                zwlr_foreign_toplevel_manager_v1::toplevel(&mut client.out, list, handle.id);
            } else {
                let handle = client.add_server(Interface::ExtForeignToplevelHandleV1, list.version, h);
                ext_foreign_toplevel_list_v1::toplevel(&mut client.out, list, handle.id);
                ext_foreign_toplevel_handle_v1::identifier(&mut client.out, handle, &mapping.to_string());
            }
        }
        let handles: Vec<(u32, u32, u64)> = client
            .objects()
            .filter_map(|o| match &o.state {
                Data::Handle(h) => Some((o.obj.id, h.list, h.mapping?)),
                _ => None,
            })
            .collect();
        let outputs = client.outputs();
        client.each_mut(|out, o| {
            let Data::Handle(h) = &mut o.state else { return };
            let wlr = o.interface == Interface::ZwlrForeignToplevelHandleV1;
            let Some(mapping) = h.mapping else { return };
            let Some((_, title, app_id, state, on)) = windows.iter().find(|w| w.0 == mapping) else {
                h.mapping = None;
                return match wlr {
                    true => zwlr_foreign_toplevel_handle_v1::closed(out, o.obj),
                    false => ext_foreign_toplevel_handle_v1::closed(out, o.obj),
                };
            };
            let shown = Some((title.clone(), app_id.clone(), if wlr { *state } else { Default::default() }));
            let mut changed = h.shown != shown;
            if changed && wlr {
                use zwlr_foreign_toplevel_handle_v1::state::*;
                let (activated, fullscreen, parent) = *state;
                let states = [activated.then_some(ACTIVATED), fullscreen.then_some(FULLSCREEN)];
                let states: Vec<u8> = states.into_iter().flatten().flat_map(u32::to_ne_bytes).collect();
                let parent = handles.iter().find(|x| x.1 == h.list && Some(x.2) == parent).map(|x| x.0);
                zwlr_foreign_toplevel_handle_v1::title(out, o.obj, title);
                zwlr_foreign_toplevel_handle_v1::app_id(out, o.obj, app_id);
                zwlr_foreign_toplevel_handle_v1::state(out, o.obj, &states);
                zwlr_foreign_toplevel_handle_v1::parent(out, o.obj, parent);
            } else if changed {
                ext_foreign_toplevel_handle_v1::title(out, o.obj, title);
                ext_foreign_toplevel_handle_v1::app_id(out, o.obj, app_id);
            }
            h.shown = shown;
            if wlr {
                changed |= client::enter(&mut h.entered, &outputs, on, |id, enter| match enter {
                    true => zwlr_foreign_toplevel_handle_v1::output_enter(out, o.obj, id),
                    false => zwlr_foreign_toplevel_handle_v1::output_leave(out, o.obj, id),
                });
            }
            match (changed, wlr) {
                (true, true) => zwlr_foreign_toplevel_handle_v1::done(out, o.obj),
                (true, false) => ext_foreign_toplevel_handle_v1::done(out, o.obj),
                _ => {}
            }
        });
    }
}

impl zwlr_foreign_toplevel_manager_v1::Handler for State {
    fn stop(&mut self, cx: Cx) {
        crate::client::send_destructor(self, cx.client, cx.obj, zwlr_foreign_toplevel_manager_v1::finished);
    }
}

impl ext_foreign_toplevel_list_v1::Handler for State {
    fn stop(&mut self, cx: Cx) {
        self.set(cx.key(), Data::None);
        self.send(cx.client, |out| ext_foreign_toplevel_list_v1::finished(out, cx.obj));
    }
}

fn request(st: &mut State, cx: Cx, f: impl FnOnce(&mut State, WindowId)) -> Res {
    if let Some(w) = window_of(st, cx.key()) {
        f(st, w);
        st.shell.dirty = true;
    }
    Ok(())
}

impl zwlr_foreign_toplevel_handle_v1::Handler for State {
    fn set_maximized(&mut self, _: Cx) {}

    fn unset_maximized(&mut self, _: Cx) {}

    fn set_minimized(&mut self, _: Cx) {}

    fn unset_minimized(&mut self, _: Cx) {}

    fn activate(&mut self, cx: Cx, _: u32) -> Res {
        request(self, cx, |st, w| st.policy.wm().focus(w, Cause::User))
    }

    fn close(&mut self, cx: Cx) -> Res {
        request(self, cx, xdg::close)
    }

    fn set_rectangle(&mut self, _: Cx, _: u32, _: i32, _: i32, _: i32, _: i32) {}

    fn set_fullscreen(&mut self, cx: Cx, output: Option<u32>) -> Res {
        let output = output.and_then(|o| self.client(cx.client).output(o));
        request(self, cx, |st, w| st.policy.wm().fullscreen(w, true, output))
    }

    fn unset_fullscreen(&mut self, cx: Cx) -> Res {
        request(self, cx, |st, w| st.policy.wm().fullscreen(w, false, None))
    }
}

impl ext_foreign_toplevel_handle_v1::Handler for State {}

pub fn window_of(st: &State, handle: Key) -> Option<WindowId> {
    let Data::Handle(h) = st.data(handle)? else { return None };
    xdg::managed_windows(st).find(|w| Some(w.1) == h.mapping).map(|w| w.0)
}
