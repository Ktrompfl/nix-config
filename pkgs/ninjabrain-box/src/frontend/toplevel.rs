//! Knowing which window is focused, so that the overlay can stay out of the
//! way of everything that is not the game.
//!
//! `zwlr_foreign_toplevel_management_v1` is the one that reports activation;
//! the newer `ext_foreign_toplevel_list_v1` only enumerates windows and cannot
//! say which has focus, so it is no use here. Compositors without it (GNOME)
//! simply lose the filter.

use std::collections::HashMap;

use wayland_client::backend::ObjectId;
use wayland_client::globals::GlobalList;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols_wlr::foreign_toplevel::v1::client::{
    zwlr_foreign_toplevel_handle_v1::{self, State, ZwlrForeignToplevelHandleV1},
    zwlr_foreign_toplevel_manager_v1::{self, ZwlrForeignToplevelManagerV1},
};

use super::app::App;

#[derive(Default)]
struct Toplevel {
    app_id: String,
    title: String,
    activated: bool,
}

/// Tracks the focused window's identity.
pub struct Focus {
    _manager: ZwlrForeignToplevelManagerV1,
    toplevels: HashMap<ObjectId, Toplevel>,
}

impl Focus {
    pub fn new(globals: &GlobalList, queue: &QueueHandle<App>) -> Option<Focus> {
        let manager = globals
            .bind::<ZwlrForeignToplevelManagerV1, _, _>(queue, 1..=3, ())
            .ok()?;
        Some(Focus {
            _manager: manager,
            toplevels: HashMap::new(),
        })
    }

    /// Whether the focused window matches any of `patterns`, as a substring of
    /// either its app id or its title.
    pub fn matches(&self, patterns: &[String]) -> bool {
        self.toplevels.values().any(|toplevel| {
            toplevel.activated
                && patterns
                    .iter()
                    .any(|pattern| {
                        toplevel.app_id.contains(pattern.as_str())
                            || toplevel.title.contains(pattern.as_str())
                    })
        })
    }
}

impl Dispatch<ZwlrForeignToplevelManagerV1, ()> for App {
    fn event(
        _: &mut App,
        _: &ZwlrForeignToplevelManagerV1,
        _: zwlr_foreign_toplevel_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<App>,
    ) {
    }

    wayland_client::event_created_child!(App, ZwlrForeignToplevelManagerV1, [
        zwlr_foreign_toplevel_manager_v1::EVT_TOPLEVEL_OPCODE => (ZwlrForeignToplevelHandleV1, ()),
    ]);
}

impl Dispatch<ZwlrForeignToplevelHandleV1, ()> for App {
    fn event(
        app: &mut App,
        handle: &ZwlrForeignToplevelHandleV1,
        event: zwlr_foreign_toplevel_handle_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<App>,
    ) {
        let Some(focus) = app.focus.as_mut() else {
            return;
        };
        let id = handle.id();
        match event {
            zwlr_foreign_toplevel_handle_v1::Event::AppId { app_id } => {
                focus.toplevels.entry(id).or_default().app_id = app_id;
            }
            zwlr_foreign_toplevel_handle_v1::Event::Title { title } => {
                focus.toplevels.entry(id).or_default().title = title;
            }
            zwlr_foreign_toplevel_handle_v1::Event::State { state } => {
                let activated = state
                    .chunks_exact(4)
                    .filter_map(|bytes| {
                        State::try_from(u32::from_ne_bytes(bytes.try_into().ok()?)).ok()
                    })
                    .any(|state| state == State::Activated);
                focus.toplevels.entry(id).or_default().activated = activated;
            }
            zwlr_foreign_toplevel_handle_v1::Event::Closed => {
                focus.toplevels.remove(&id);
                handle.destroy();
            }
            zwlr_foreign_toplevel_handle_v1::Event::Done => app.refresh_visibility(),
            _ => {}
        }
    }
}
