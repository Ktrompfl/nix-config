use crate::{
    State, caps,
    client::{self, Data},
};
use backend::OutputId;
use wire::{ext_workspace_handle_v1::workspace_capabilities::ACTIVATE, *};

#[derive(Default)]
pub struct Manager {
    names: Vec<String>,
    outputs: Vec<OutputId>,
    pending: Vec<String>,
}

pub struct Group {
    manager: u32,
    output: OutputId,
    entered: Vec<u32>,
}

pub struct Workspace {
    manager: u32,
    name: String,
    group: Option<Obj>,
    shown: Option<(u32, u32)>,
}

pub fn sync(st: &mut State) {
    let (list, outputs) = (st.policy.wm().workspaces(), crate::output::in_layout(st));
    let names: Vec<String> = list.iter().map(|w| w.name.clone()).collect();
    for client in st.clients.values_mut().filter(|c| c.caps & caps::WORKSPACES != 0) {
        let (mut managers, mut new, mut changed) = (vec![], vec![], vec![]);
        client.each_mut(|_, o| {
            if let Data::Workspaces(m) = &mut o.state {
                managers.push(o.obj);
                new.extend(outputs.iter().filter(|x| !m.outputs.contains(x)).map(|&x| (o.obj, Ok(x))));
                new.extend(names.iter().filter(|w| !m.names.contains(w)).map(|w| (o.obj, Err(w.clone()))));
                (m.outputs, m.names) = (outputs.clone(), names.clone());
            }
        });
        for (manager, new) in new {
            changed.push(manager.id);
            match new {
                Ok(output) => {
                    let g = Data::WorkspaceGroup(Group { manager: manager.id, output, entered: vec![] });
                    let group = client.add_server(Interface::ExtWorkspaceGroupHandleV1, manager.version, g);
                    ext_workspace_manager_v1::workspace_group(&mut client.out, manager, group.id);
                    ext_workspace_group_handle_v1::capabilities(&mut client.out, group, 0);
                }
                Err(name) => {
                    let w = Workspace { manager: manager.id, name: name.clone(), group: None, shown: None };
                    let workspace =
                        client.add_server(Interface::ExtWorkspaceHandleV1, manager.version, Data::Workspace(w));
                    let out = &mut client.out;
                    ext_workspace_manager_v1::workspace(out, manager, workspace.id);
                    ext_workspace_handle_v1::id(out, workspace, &name);
                    ext_workspace_handle_v1::name(out, workspace, &name);
                    ext_workspace_handle_v1::capabilities(out, workspace, ACTIVATE);
                }
            }
        }
        let groups: Vec<(Obj, u32, OutputId)> = client
            .objects()
            .filter_map(|o| match &o.state {
                Data::WorkspaceGroup(g) => Some((o.obj, g.manager, g.output)),
                _ => None,
            })
            .collect();
        let live = |manager: u32| managers.iter().any(|m| m.id == manager);
        client.each_mut(|out, o| {
            let Data::Workspace(w) = &mut o.state else { return };
            if !live(w.manager) {
                return;
            }
            let workspace = list.iter().enumerate().find(|x| x.1.name == w.name);
            let wanted = workspace.and_then(|x| groups.iter().find(|g| g.1 == w.manager && Some(g.2) == x.1.output));
            let wanted = wanted.filter(|g| outputs.contains(&g.2)).map(|g| g.0);
            if w.group != wanted {
                if let Some(group) = w.group.filter(|g| groups.iter().any(|x| x.0 == *g)) {
                    ext_workspace_group_handle_v1::workspace_leave(out, group, o.obj.id);
                }
                if let Some(group) = wanted {
                    ext_workspace_group_handle_v1::workspace_enter(out, group, o.obj.id);
                }
                w.group = wanted;
                changed.push(w.manager);
            }
            use ext_workspace_handle_v1::state::*;
            let shown = workspace.map(|(order, x)| {
                (if x.active { ACTIVE } else { 0 } | if x.urgent { URGENT } else { 0 }, order as u32)
            });
            match shown {
                None => {
                    ext_workspace_handle_v1::removed(out, o.obj);
                    changed.push(w.manager);
                    o.state = Data::None;
                }
                Some((state, order)) if w.shown != shown => {
                    ext_workspace_handle_v1::state(out, o.obj, state);
                    ext_workspace_handle_v1::coordinates(out, o.obj, &order.to_ne_bytes());
                    w.shown = shown;
                    changed.push(w.manager);
                }
                _ => {}
            }
        });
        let wl_outputs = client.outputs();
        client.each_mut(|out, o| {
            let Data::WorkspaceGroup(g) = &mut o.state else { return };
            if !live(g.manager) {
                return;
            }
            if !outputs.contains(&g.output) {
                changed.push(g.manager);
                ext_workspace_group_handle_v1::removed(out, o.obj);
                o.state = Data::None;
            } else if client::enter(&mut g.entered, &wl_outputs, &[g.output], |id, enter| match enter {
                true => ext_workspace_group_handle_v1::output_enter(out, o.obj, id),
                false => ext_workspace_group_handle_v1::output_leave(out, o.obj, id),
            }) {
                changed.push(g.manager);
            }
        });
        for manager in managers.into_iter().filter(|m| changed.contains(&m.id)) {
            ext_workspace_manager_v1::done(&mut client.out, manager);
        }
    }
}

impl ext_workspace_manager_v1::Handler for State {
    fn commit(&mut self, cx: Cx) {
        let Some(Data::Workspaces(manager)) = self.data_mut(cx.key()) else { return };
        for name in std::mem::take(&mut manager.pending) {
            self.policy.wm().activate_workspace(&name);
            self.shell.dirty = true;
        }
    }

    fn stop(&mut self, cx: Cx) {
        crate::client::send_destructor(self, cx.client, cx.obj, ext_workspace_manager_v1::finished);
    }
}

impl ext_workspace_handle_v1::Handler for State {
    fn activate(&mut self, cx: Cx) {
        let Some(Data::Workspace(w)) = self.data(cx.key()) else { return };
        let (manager, name) = ((cx.client, w.manager), w.name.clone());
        if let Some(Data::Workspaces(m)) = self.data_mut(manager) {
            m.pending.push(name);
        }
    }

    fn deactivate(&mut self, _: Cx) {}

    fn assign(&mut self, _: Cx, _: u32) {}

    fn remove(&mut self, _: Cx) {}
}

impl ext_workspace_group_handle_v1::Handler for State {
    fn create_workspace(&mut self, _: Cx, _: String) {}
}
