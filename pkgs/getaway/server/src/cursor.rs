use crate::{
    Key, State,
    scene::Item,
    surface::{self, Role},
};
use gfx::{Pixels, Renderer, TexId};
use std::{cell::Cell, collections::HashMap, env, fs, rc::Rc};
use wire::*;
use wm::Rect;

const SHAPES: [&str; 36] = [
    "default",
    "context-menu",
    "help",
    "pointer",
    "progress",
    "wait",
    "cell",
    "crosshair",
    "text",
    "vertical-text",
    "alias",
    "copy",
    "move",
    "no-drop",
    "not-allowed",
    "grab",
    "grabbing",
    "e-resize",
    "n-resize",
    "ne-resize",
    "nw-resize",
    "s-resize",
    "se-resize",
    "sw-resize",
    "w-resize",
    "ew-resize",
    "ns-resize",
    "nesw-resize",
    "nwse-resize",
    "col-resize",
    "row-resize",
    "all-scroll",
    "zoom-in",
    "zoom-out",
    "dnd-ask",
    "all-resize",
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Cursor {
    Hidden,
    Surface { key: Key, hotspot: (i32, i32) },
    Named(&'static str),
}

pub struct CursorImage {
    pub pixels: Rc<Pixels>,
    pub hotspot: (i32, i32),
    pub id: u64,
    texture: Cell<Option<TexId>>,
}

impl CursorImage {
    pub fn texture(&self, renderer: &mut Renderer) -> Option<TexId> {
        if let Some(texture) = self.texture.get() {
            return Some(texture);
        }
        let texture =
            renderer.upload(None, &*self.pixels).inspect_err(|e| log::warn!("cannot upload cursor: {e}")).ok();
        self.texture.set(texture);
        texture
    }
}

pub struct Theme {
    name: String,
    size: u32,
    cache: HashMap<(&'static str, u32), Option<Rc<CursorImage>>>,
}

impl Default for Theme {
    fn default() -> Self {
        let name = env::var("XCURSOR_THEME").unwrap_or_else(|_| "default".into());
        let size = env::var("XCURSOR_SIZE").ok().and_then(|s| s.parse().ok()).unwrap_or(24);
        Self { name, size, cache: HashMap::new() }
    }
}

impl Theme {
    fn image(&mut self, name: &'static str, scale: f64) -> Option<Rc<CursorImage>> {
        let size = (f64::from(self.size) * scale).round() as u32;
        let theme = &self.name;
        self.cache
            .entry((name, size))
            .or_insert_with(|| load(theme, name, size).or_else(|| load(theme, "default", size)))
            .clone()
    }
}

fn load(theme: &str, name: &str, size: u32) -> Option<Rc<CursorImage>> {
    let bytes = fs::read(xcursor::CursorTheme::load(theme).load_icon(name)?).ok()?;
    let images = xcursor::parser::parse_xcursor(&bytes)?;
    let best = images.iter().min_by_key(|i| i.size.abs_diff(size))?;
    let data = best.pixels_rgba.as_chunks::<4>().0.iter().map(|&p| u32::from_le_bytes(p)).collect();
    let pixels = Rc::new(Pixels { data, width: best.width as i32, height: best.height as i32 });
    let (hotspot, id, texture) = ((best.xhot as i32, best.yhot as i32), crate::render::content_id(), Cell::new(None));
    Some(Rc::new(CursorImage { pixels, hotspot, id, texture }))
}

impl wl_pointer::Handler for State {
    fn set_cursor(&mut self, cx: Cx, serial: u32, surface: Option<u32>, hotspot_x: i32, hotspot_y: i32) -> Res {
        let c = cx.client;
        if self.seat.pointer_client() != Some(c) || serial != self.seat.enter_serial() {
            return Ok(());
        }
        self.cursor = match surface {
            None => Cursor::Hidden,
            Some(s) => {
                let key = (c, s);
                if !matches!(self.surfaces.get(&key).map(|s| &s.role), Some(Role::Cursor)) {
                    surface::assign_role(self, key, Role::Cursor).map_err(|m| cx.error(0, m))?;
                }
                Cursor::Surface { key, hotspot: (hotspot_x, hotspot_y) }
            }
        };
        self.render_dirty = true;
        Ok(())
    }
}

pub fn offset(st: &mut State, key: Key, (dx, dy): (i32, i32)) {
    if let Cursor::Surface { key: k, hotspot } = &mut st.cursor
        && *k == key
    {
        *hotspot = (hotspot.0 - dx, hotspot.1 - dy);
    }
}

impl wp_cursor_shape_manager_v1::Handler for State {
    fn get_pointer(&mut self, _: Cx, _: Obj, _: u32) {}

    fn get_tablet_tool_v2(&mut self, _: Cx, _: Obj, _: u32) {}
}

impl wp_cursor_shape_device_v1::Handler for State {
    fn set_shape(&mut self, cx: Cx, serial: u32, shape: u32) -> Res {
        let count = if cx.obj.version >= 2 { SHAPES.len() } else { SHAPES.len() - 2 };
        let Some(&name) = SHAPES[..count].get(shape.wrapping_sub(1) as usize) else {
            return Err(cx.error(wp_cursor_shape_device_v1::error::INVALID_SHAPE, "invalid shape"));
        };
        if self.seat.pointer_client() == Some(cx.client) && serial == self.seat.enter_serial() {
            self.cursor = Cursor::Named(name);
            self.render_dirty = true;
        }
        Ok(())
    }
}

pub fn item(st: &mut State, output: backend::OutputId) -> Option<Item> {
    let (x, y) = st.seat.pointer()?;
    let (rect, scale) = crate::output::find(st, output).filter(|o| o.rect.contains(x, y)).map(|o| (o.rect, o.scale))?;
    match st.cursor {
        Cursor::Hidden => None,
        Cursor::Surface { key, hotspot } => {
            let origin = (x.round() as i32 - hotspot.0, y.round() as i32 - hotspot.1);
            st.surfaces.get(&key)?.buffer.as_ref()?;
            Some(Item::Surface { key, origin, clip: rect })
        }
        Cursor::Named(name) => {
            let image = st.cursor_theme.image(name, scale)?;
            let local = |v: f64, origin: i32| ((v - f64::from(origin)) * scale).round() as i32;
            let at = (local(x, rect.x) - image.hotspot.0, local(y, rect.y) - image.hotspot.1);
            let dst = Rect::new(at.0, at.1, image.pixels.width, image.pixels.height);
            Some(Item::Image { image, dst })
        }
    }
}
