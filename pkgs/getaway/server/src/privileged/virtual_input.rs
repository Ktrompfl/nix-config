use crate::{Key, State, client::Data, output, seat};
use backend::{OutputId, Scroll, ScrollAxis};
use kbvm::lookup::LookupTable;
use std::{fs::File, os::fd::OwnedFd, os::unix::fs::FileExt};

const MAX_KEYMAP: u32 = 1 << 20;
use wire::*;

pub struct VirtualKeyboard {
    pub keymap: (OwnedFd, u32),
    pub lookup: Option<LookupTable>,
    pub mods: [u32; 4],
}

#[derive(Default)]
pub struct VirtualPointer {
    output: Option<OutputId>,
    scroll: Option<Scroll>,
}

impl zwp_virtual_keyboard_manager_v1::Handler for State {
    fn create_virtual_keyboard(&mut self, _: Cx, _: u32, _: Obj) {}
}

fn missing(cx: Cx) -> Error {
    cx.error(zwp_virtual_keyboard_v1::error::NO_KEYMAP, "no keymap set")
}

impl zwp_virtual_keyboard_v1::Handler for State {
    fn keymap(&mut self, cx: Cx, _: u32, fd: OwnedFd, size: u32) {
        let Some((fd, keymap)) = Some(size).filter(|&s| s <= MAX_KEYMAP).and_then(|s| copy_keymap(&fd, s)) else {
            log::warn!("ignoring invalid virtual keyboard keymap");
            return;
        };
        let lookup = Some(keymap.to_builder().build_lookup_table());
        let mods = keyboard(self, cx.key()).map_or([0; 4], |v| v.mods);
        self.set(cx.key(), Data::VirtualKeyboard(VirtualKeyboard { keymap: (fd, size), lookup, mods }));
        seat::virtual_keymap_changed(self, cx.key());
    }

    fn key(&mut self, cx: Cx, _: u32, key: u32, state: u32) -> Res {
        keyboard(self, cx.key()).ok_or_else(|| missing(cx))?;
        seat::virtual_key(self, cx.key(), key, state == wl_keyboard::key_state::PRESSED);
        Ok(())
    }

    fn modifiers(&mut self, cx: Cx, depressed: u32, latched: u32, locked: u32, group: u32) -> Res {
        let Some(Data::VirtualKeyboard(keyboard)) = self.data_mut(cx.key()) else { return Err(missing(cx)) };
        keyboard.mods = [depressed, latched, locked, group];
        seat::virtual_modifiers(self, cx.key());
        Ok(())
    }
}

pub fn keyboard(st: &State, key: Key) -> Option<&VirtualKeyboard> {
    match &st.clients.get(&key.0)?.get(key.1)?.state {
        Data::VirtualKeyboard(v) => Some(v),
        _ => None,
    }
}

fn copy_keymap(fd: &OwnedFd, size: u32) -> Option<(OwnedFd, kbvm::xkb::Keymap)> {
    let mut text = vec![0; size as usize];
    File::from(fd.try_clone().ok()?).read_exact_at(&mut text, 0).ok()?;
    let keymap = seat::parse_keymap_text(&text)?;
    let copy = rustix::fs::memfd_create("getaway-keymap", rustix::fs::MemfdFlags::CLOEXEC).ok()?;
    File::from(copy.try_clone().ok()?).write_all_at(&text, 0).ok()?;
    Some((copy, keymap))
}

impl zwlr_virtual_pointer_manager_v1::Handler for State {
    fn create_virtual_pointer(&mut self, cx: Cx, _: Option<u32>, id: Obj) {
        virtual_pointer(self, cx, id, None);
    }

    fn create_virtual_pointer_with_output(&mut self, cx: Cx, _: Option<u32>, output: Option<u32>, id: Obj) {
        let output = output.and_then(|o| self.client(cx.client).output(o));
        virtual_pointer(self, cx, id, output);
    }
}

fn virtual_pointer(st: &mut State, cx: Cx, id: Obj, output: Option<OutputId>) {
    st.set((cx.client, id.id), Data::VirtualPointer(VirtualPointer { output, scroll: None }));
}

fn check_axis(cx: Cx, axis: u32) -> Res {
    match axis {
        0 | 1 => Ok(()),
        _ => Err(cx.error(zwlr_virtual_pointer_v1::error::INVALID_AXIS, "invalid axis")),
    }
}

impl zwlr_virtual_pointer_v1::Handler for State {
    fn motion(&mut self, _: Cx, _: u32, dx: Fixed, dy: Fixed) {
        seat::move_pointer(self, dx.into(), dy.into());
    }

    fn motion_absolute(&mut self, cx: Cx, _: u32, x: u32, y: u32, x_extent: u32, y_extent: u32) {
        let Some(Data::VirtualPointer(p)) = self.data_mut(cx.key()) else { return };
        let output = p.output;
        let bounds = output.and_then(|o| output::find(self, o)).map(|o| o.rect).or_else(|| output::bounds(self));
        if let Some(r) = bounds.filter(|_| x_extent > 0 && y_extent > 0) {
            let at = |v: u32, extent: u32, start: i32, len: i32| {
                f64::from(start) + f64::from(v) / f64::from(extent) * f64::from(len)
            };
            seat::motion(self, at(x, x_extent, r.x, r.w), at(y, y_extent, r.y, r.h));
        }
    }

    fn button(&mut self, _: Cx, _: u32, button: u32, state: u32) {
        seat::button(self, button, state == wl_pointer::button_state::PRESSED);
    }

    fn axis(&mut self, cx: Cx, _: u32, index: u32, value: Fixed) -> Res {
        check_axis(cx, index)?;
        scroll(self, cx, axis(index, |a| a.value = value.into()));
        Ok(())
    }

    fn frame(&mut self, cx: Cx) {
        if let Some(Data::VirtualPointer(p)) = self.data_mut(cx.key())
            && let Some(scroll) = p.scroll.take()
        {
            seat::scroll(self, scroll);
        }
    }

    fn axis_source(&mut self, cx: Cx, axis_source: u32) -> Res {
        cx.error_if(axis_source > 3, zwlr_virtual_pointer_v1::error::INVALID_AXIS_SOURCE, "invalid axis source")?;
        scroll(self, cx, |s| s.source = axis_source);
        Ok(())
    }

    fn axis_stop(&mut self, cx: Cx, _: u32, index: u32) -> Res {
        check_axis(cx, index)?;
        scroll(self, cx, axis(index, |a| a.value = 0.0));
        Ok(())
    }

    fn axis_discrete(&mut self, cx: Cx, _: u32, index: u32, value: Fixed, discrete: i32) -> Res {
        check_axis(cx, index)?;
        scroll(self, cx, axis(index, move |a| (a.value, a.value120) = (value.into(), discrete * 120)));
        Ok(())
    }
}

fn scroll(st: &mut State, cx: Cx, f: impl FnOnce(&mut Scroll)) {
    if let Some(Data::VirtualPointer(p)) = st.data_mut(cx.key()) {
        f(p.scroll.get_or_insert_default());
    }
}

fn axis(index: u32, f: impl FnOnce(&mut ScrollAxis)) -> impl FnOnce(&mut Scroll) {
    move |s| f(s.axes[index as usize].get_or_insert_default())
}
