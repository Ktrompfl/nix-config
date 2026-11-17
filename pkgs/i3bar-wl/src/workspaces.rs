use crate::App;
use smithay_client_toolkit::{
    dispatch2::Dispatch2,
    reexports::client::{Connection, QueueHandle, WEnum, backend::ObjectData, protocol::wl_output::WlOutput},
};
use std::sync::Arc;
use wayland_protocols::ext::workspace::v1::client::{
    ext_workspace_group_handle_v1::{self, ExtWorkspaceGroupHandleV1},
    ext_workspace_handle_v1::{self, ExtWorkspaceHandleV1, State},
    ext_workspace_manager_v1::{self, ExtWorkspaceManagerV1},
};

pub struct Data;

#[derive(Default)]
pub struct Workspaces {
    pub manager: Option<ExtWorkspaceManagerV1>,
    groups: Vec<Group>,
    list: Vec<Workspace>,
}

struct Group {
    handle: ExtWorkspaceGroupHandleV1,
    outputs: Vec<WlOutput>,
    members: Vec<ExtWorkspaceHandleV1>,
}

pub struct Workspace {
    pub handle: ExtWorkspaceHandleV1,
    pub name: String,
    pub state: State,
    order: Vec<u32>,
}

impl Workspaces {
    pub fn on(&self, output: &WlOutput) -> Vec<&Workspace> {
        let groups = self.groups.iter().filter(|g| g.outputs.contains(output));
        let members = groups.flat_map(|g| &g.members);
        let mut shown: Vec<&Workspace> = members
            .filter_map(|m| self.list.iter().find(|w| &w.handle == m))
            .filter(|w| !w.state.contains(State::Hidden))
            .collect();
        shown.sort_by(|a, b| (&a.order, &a.name).cmp(&(&b.order, &b.name)));
        shown
    }

    pub fn activate(&self, handle: &ExtWorkspaceHandleV1) {
        if let Some(manager) = &self.manager {
            handle.activate();
            manager.commit();
        }
    }

    fn group(&mut self, handle: &ExtWorkspaceGroupHandleV1) -> Option<&mut Group> {
        self.groups.iter_mut().find(|g| &g.handle == handle)
    }
}

impl Dispatch2<ExtWorkspaceManagerV1, App> for Data {
    fn event(
        &self,
        app: &mut App,
        _: &ExtWorkspaceManagerV1,
        event: ext_workspace_manager_v1::Event,
        _: &Connection,
        _: &QueueHandle<App>,
    ) {
        let w = &mut app.workspaces;
        match event {
            ext_workspace_manager_v1::Event::WorkspaceGroup { workspace_group } => {
                w.groups.push(Group { handle: workspace_group, outputs: vec![], members: vec![] })
            }
            ext_workspace_manager_v1::Event::Workspace { workspace } => {
                w.list.push(Workspace { handle: workspace, name: String::new(), state: State::empty(), order: vec![] })
            }
            ext_workspace_manager_v1::Event::Done => app.dirty = true,
            ext_workspace_manager_v1::Event::Finished => w.manager = None,
            _ => {}
        }
    }

    fn event_created_child(opcode: u16, qh: &QueueHandle<App>) -> Arc<dyn ObjectData> {
        match opcode {
            ext_workspace_manager_v1::EVT_WORKSPACE_GROUP_OPCODE => qh.make_data::<ExtWorkspaceGroupHandleV1, _>(Data),
            _ => qh.make_data::<ExtWorkspaceHandleV1, _>(Data),
        }
    }
}

impl Dispatch2<ExtWorkspaceGroupHandleV1, App> for Data {
    fn event(
        &self,
        app: &mut App,
        handle: &ExtWorkspaceGroupHandleV1,
        event: ext_workspace_group_handle_v1::Event,
        _: &Connection,
        _: &QueueHandle<App>,
    ) {
        let w = &mut app.workspaces;
        match event {
            ext_workspace_group_handle_v1::Event::OutputEnter { output } => {
                w.group(handle).into_iter().for_each(|g| g.outputs.push(output.clone()))
            }
            ext_workspace_group_handle_v1::Event::OutputLeave { output } => {
                w.group(handle).into_iter().for_each(|g| g.outputs.retain(|o| o != &output))
            }
            ext_workspace_group_handle_v1::Event::WorkspaceEnter { workspace } => {
                w.group(handle).into_iter().for_each(|g| g.members.push(workspace.clone()))
            }
            ext_workspace_group_handle_v1::Event::WorkspaceLeave { workspace } => {
                w.group(handle).into_iter().for_each(|g| g.members.retain(|m| m != &workspace))
            }
            ext_workspace_group_handle_v1::Event::Removed => {
                w.groups.retain(|g| &g.handle != handle);
                handle.destroy();
            }
            _ => {}
        }
    }
}

impl Dispatch2<ExtWorkspaceHandleV1, App> for Data {
    fn event(
        &self,
        app: &mut App,
        handle: &ExtWorkspaceHandleV1,
        event: ext_workspace_handle_v1::Event,
        _: &Connection,
        _: &QueueHandle<App>,
    ) {
        let w = &mut app.workspaces;
        let Some(index) = w.list.iter().position(|w| &w.handle == handle) else { return };
        match event {
            ext_workspace_handle_v1::Event::Name { name } => w.list[index].name = name,
            ext_workspace_handle_v1::Event::Coordinates { coordinates } => {
                w.list[index].order = coordinates.as_chunks::<4>().0.iter().map(|c| u32::from_ne_bytes(*c)).collect()
            }
            ext_workspace_handle_v1::Event::State { state: WEnum::Value(state) } => w.list[index].state = state,
            ext_workspace_handle_v1::Event::Removed => {
                w.list.remove(index);
                w.groups.iter_mut().for_each(|g| g.members.retain(|m| m != handle));
                handle.destroy();
            }
            _ => {}
        }
    }
}
