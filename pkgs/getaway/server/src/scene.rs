use crate::{
    Key, State,
    privileged::{
        layer_shell::{self, LayerKind},
        session_lock,
    },
    surface::Role,
};
use std::rc::Rc;
use wm::{Color, Deco, Rect, WindowId};

const LOCKED_BACKGROUND: Color = 0xff000000;

#[derive(Clone)]
pub enum Item {
    Fill(Rect, Color),
    Surface { key: Key, origin: (i32, i32), clip: Rect },
    Ghost { window: WindowId, rect: Rect },
    Text { rect: Rect, text: Rc<str>, font: Rc<str>, fg: Color, bg: Color },
    Image { image: Rc<crate::cursor::CursorImage>, dst: Rect },
}

pub fn build(st: &State) -> Vec<Item> {
    if session_lock::locked(st) {
        let fills = enabled_outputs(st).map(|o| Item::Fill(o.rect, LOCKED_BACKGROUND));
        let surfaces = session_lock::surfaces(st).into_iter().filter_map(|(key, output)| {
            let rect = crate::output::find(st, output)?.rect;
            Some(Item::Surface { key, origin: (rect.x, rect.y), clip: rect })
        });
        return fills.chain(surfaces).collect();
    }
    let shown = &st.shell.shown;
    let fullscreen = shown.windows.iter().any(|p| p.fullscreen);
    let mut items: Vec<Item> = enabled_outputs(st).map(|o| Item::Fill(o.rect, shown.background)).collect();
    push_layers(st, LayerKind::Background, &mut items);
    push_layers(st, LayerKind::Bottom, &mut items);
    items.extend(shown.decorations.iter().map(|deco| match deco {
        Deco::Fill(rect, color) => Item::Fill(*rect, *color),
        Deco::Text { rect, text, font, fg, bg } => {
            Item::Text { rect: *rect, text: text.clone(), font: font.clone(), fg: *fg, bg: *bg }
        }
    }));
    if fullscreen {
        push_layers(st, LayerKind::Top, &mut items);
    }
    for p in &shown.windows {
        let bare = |key| st.surfaces.get(key).is_none_or(|s| s.buffer.is_none());
        items.push(match st.windows.get(&p.window) {
            Some(key) if bare(key) && st.shell.ghosts.contains_key(&p.window) => {
                Item::Ghost { window: p.window, rect: p.rect }
            }
            Some(&key) => {
                Item::Surface { key, origin: surface_origin(st, key).unwrap_or((p.rect.x, p.rect.y)), clip: p.rect }
            }
            None => Item::Ghost { window: p.window, rect: p.rect },
        });
    }
    push_popups(st, false, &mut items);
    if !fullscreen {
        push_layers(st, LayerKind::Top, &mut items);
    }
    push_layers(st, LayerKind::Overlay, &mut items);
    push_popups(st, true, &mut items);
    items
}

fn push_popups(st: &State, of_layers: bool, items: &mut Vec<Item>) {
    for &key in st.popups.iter().filter(|&&k| matches!(root_role(st, k), Some(Role::Layer(_))) == of_layers) {
        if let (Some(origin), Some(Role::Popup(p))) = (surface_origin(st, key), st.surfaces.get(&key).map(|s| &s.role))
        {
            let (x, y) = geometry_origin(st, key).unwrap_or(origin);
            items.push(Item::Surface { key, origin, clip: Rect { x, y, ..p.rect } });
        }
    }
}

fn root_role(st: &State, key: Key) -> Option<&Role> {
    match &st.surfaces.get(&key)?.role {
        Role::Popup(p) => root_role(st, (key.0, p.parent?)),
        role => Some(role),
    }
}

fn push_layers(st: &State, kind: LayerKind, items: &mut Vec<Item>) {
    for &key in &st.layers {
        if let Some(rect) = layer_shell::visible(st, key, kind) {
            items.push(Item::Surface { key, origin: (rect.x, rect.y), clip: rect });
        }
    }
}

pub fn geometry_origin(st: &State, key: Key) -> Option<(i32, i32)> {
    match &st.surfaces.get(&key)?.role {
        Role::Toplevel(t) => {
            st.shell.shown.windows.iter().find(|p| p.window == t.window()).map(|p| (p.rect.x, p.rect.y))
        }
        Role::Popup(p) => {
            let (x, y) = geometry_origin(st, (key.0, p.parent?))?;
            Some((x + p.rect.x, y + p.rect.y))
        }
        Role::Layer(l) => Some((l.rect.x, l.rect.y)),
        Role::LockSurface(l) => crate::output::find(st, l.output).map(|o| (o.rect.x, o.rect.y)),
        &Role::Subsurface { parent, x, y, .. } => surface_origin(st, (key.0, parent)).map(|(px, py)| (px + x, py + y)),
        _ => None,
    }
}

fn enabled_outputs(st: &State) -> impl Iterator<Item = &crate::output::Output> {
    st.outputs.iter().filter(|o| o.in_layout())
}

pub fn surface_rect(st: &State, key: Key) -> Option<Rect> {
    let (x, y) = surface_origin(st, key)?;
    let (w, h) = st.surfaces.get(&key)?.size();
    Some(Rect::new(x, y, w, h))
}

pub fn visible_rect(st: &State, key: Key) -> Option<Rect> {
    let rect = surface_rect(st, key)?;
    Some(window_clip(st, key).map_or(rect, |clip| rect.intersect(clip)))
}

fn window_clip(st: &State, key: Key) -> Option<Rect> {
    match &st.surfaces.get(&key)?.role {
        Role::Toplevel(t) => st.shell.shown.windows.iter().find(|p| p.window == t.window()).map(|p| p.rect),
        &Role::Subsurface { parent, .. } => window_clip(st, (key.0, parent)),
        _ => None,
    }
}

pub fn surface_origin(st: &State, key: Key) -> Option<(i32, i32)> {
    let (x, y) = geometry_origin(st, key)?;
    let g = st.surfaces.get(&key)?.geometry;
    Some((x - g.x, y - g.y))
}

pub fn surface_at(st: &State, x: f64, y: f64) -> Option<(Key, f64, f64)> {
    build(st).into_iter().rev().find_map(|item| match item {
        Item::Surface { key, origin, clip } if clip.contains(x, y) => hit(st, key, origin, (x, y)),
        _ => None,
    })
}

fn hit(st: &State, key: Key, (ox, oy): (i32, i32), (x, y): (f64, f64)) -> Option<(Key, f64, f64)> {
    let s = st.surfaces.get(&key).filter(|s| s.buffer.is_some())?;
    let below = s.below.min(s.children.len());
    let child = |&child: &u32| match st.surfaces.get(&(key.0, child)).map(|s| &s.role) {
        Some(&Role::Subsurface { x: cx, y: cy, .. }) => hit(st, (key.0, child), (ox + cx, oy + cy), (x, y)),
        _ => None,
    };
    let (lx, ly) = (x - f64::from(ox), y - f64::from(oy));
    let (w, h) = s.size();
    let inside = Rect::new(0, 0, w, h).contains(lx, ly) && s.input.as_ref().is_none_or(|r| r.contains(lx, ly));
    s.children[below..]
        .iter()
        .rev()
        .find_map(child)
        .or_else(|| inside.then_some((key, lx, ly)))
        .or_else(|| s.children[..below].iter().rev().find_map(child))
}

pub fn root(st: &State, mut key: Key) -> Key {
    while let Some(&Role::Subsurface { parent, .. }) = st.surfaces.get(&key).map(|s| &s.role) {
        key = (key.0, parent);
    }
    key
}
