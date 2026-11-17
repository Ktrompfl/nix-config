use crate::{App, Target, config::Color, status::Block, tray::Icon};
use cairo::{Context, Format, ImageSurface};
use pango::{FontDescription, Layout};
use smithay_client_toolkit::{
    reexports::client::protocol::{wl_shm, wl_surface::WlSurface},
    shell::WaylandSurface,
    shm::slot::SlotPool,
};
use std::{cell::RefCell, sync::Arc};
use wayland_protocols::{
    ext::workspace::v1::client::ext_workspace_handle_v1::State, wp::viewporter::client::wp_viewport::WpViewport,
};

const PAD: f64 = 5.0;
const ICON: f64 = 14.0;
const ICON_GAP: f64 = 4.0;

pub fn set(cr: &Context, Color(color): Color) {
    let c = |shift: u32| f64::from((color >> shift) & 0xff) / 255.0;
    cr.set_source_rgba(c(16), c(8), c(0), c(24));
}

pub fn fill(cr: &Context, (x, y, w, h): (f64, f64, f64, f64), color: Color) {
    set(cr, color);
    cr.rectangle(x, y, w, h);
    let _ = cr.fill();
}

thread_local! {
    static FONT: RefCell<FontDescription> = RefCell::new(FontDescription::from_string("sans-serif 10"));
}

pub fn set_font(font: &str) {
    FONT.set(FontDescription::from_string(font));
}

pub fn layout(cr: &Context, text: &str, markup: bool) -> Layout {
    let layout = pangocairo::functions::create_layout(cr);
    FONT.with_borrow(|font| layout.set_font_description(Some(font)));
    match markup {
        true => layout.set_markup(text),
        false => layout.set_text(text),
    }
    layout
}

pub fn text(cr: &Context, layout: &Layout, (x, y, h): (f64, f64, f64), color: Color) {
    set(cr, color);
    cr.move_to(x, y + ((h - f64::from(layout.pixel_size().1)) / 2.0).round());
    pangocairo::functions::show_layout(cr, layout);
}

pub fn measure(text: &str) -> f64 {
    let image = ImageSurface::create(Format::ARgb32, 1, 1).expect("1x1 surface");
    let cr = Context::new(&image).expect("context");
    f64::from(layout(&cr, text, false).pixel_size().0)
}

fn block_width(cr: &Context, block: &Block, layout: &Layout) -> f64 {
    let own = f64::from(layout.pixel_size().0);
    let min = match &block.min_width {
        Some(serde_json::Value::Number(n)) => n.as_f64().unwrap_or(0.0),
        Some(serde_json::Value::String(s)) => f64::from(self::layout(cr, s, false).pixel_size().0),
        _ => 0.0,
    };
    own.max(min)
}

pub fn present(
    pool: &mut SlotPool,
    (surface, viewport): (&WlSurface, Option<&WpViewport>),
    (w, h, scale): (u32, u32, u32),
    paint: impl FnOnce(&Context),
) {
    let scale_f = f64::from(scale) / 120.0;
    let (bw, bh) = ((f64::from(w) * scale_f).round().max(1.0) as i32, (f64::from(h) * scale_f).round().max(1.0) as i32);
    let Ok(stride) = Format::ARgb32.stride_for_width(bw as u32) else { return };
    let Ok((buffer, canvas)) = pool.create_buffer(bw, bh, stride, wl_shm::Format::Argb8888) else { return };
    let Ok(image) =
        (unsafe { ImageSurface::create_for_data_unsafe(canvas.as_mut_ptr(), Format::ARgb32, bw, bh, stride) })
    else {
        return;
    };
    if let Ok(cr) = Context::new(&image) {
        cr.scale(f64::from(bw) / f64::from(w), f64::from(bh) / f64::from(h));
        paint(&cr);
    }
    image.finish();
    drop(image);
    surface.damage_buffer(0, 0, bw, bh);
    if buffer.attach_to(surface).is_err() {
        return;
    }
    match viewport {
        Some(viewport) => viewport.set_destination(w as i32, h as i32),
        None => surface.set_buffer_scale((scale / 120).max(1) as i32),
    }
    surface.commit();
}

pub fn icon(icon: &Icon) -> Option<ImageSurface> {
    let data = icon.data.as_ref().clone();
    ImageSurface::create_for_data(data, Format::ARgb32, icon.width, icon.height, icon.width * 4).ok()
}

pub fn draw(app: &mut App, index: usize) {
    let bar = &app.bars[index];
    let (w, h, c) = (bar.width, app.config.height, &app.config.colors);
    if !bar.configured || w == 0 {
        return;
    }
    let mut regions = vec![];
    let surface = bar.layer.wl_surface().clone();
    let (workspaces, status, items, icons) = (&app.workspaces, &app.status, &app.items, &app.icons);
    present(&mut app.pool, (&surface, bar.viewport.as_ref()), (w, h, bar.scale), |cr| {
        let (w, h) = (f64::from(w), f64::from(h));
        fill(cr, (0.0, 0.0, w, h), c.background);
        let mut x = 0.0;
        for workspace in workspaces.on(&bar.output) {
            let (bg, fg) = match workspace.state {
                s if s.contains(State::Urgent) => (c.urgent, c.urgent_text),
                s if s.contains(State::Active) => (c.active, c.active_text),
                _ => (c.workspace, c.workspace_text),
            };
            let layout = layout(cr, &workspace.name, false);
            let width = f64::from(layout.pixel_size().0) + 2.0 * PAD;
            fill(cr, (x, 0.0, width, h), bg);
            text(cr, &layout, (x + PAD, 0.0, h), fg);
            regions.push((x, x + width, Target::Workspace(workspace.handle.clone())));
            x += width;
        }
        let mut right = w;
        for (i, item) in items.iter().enumerate().rev().filter(|(_, item)| !item.passive) {
            right -= ICON + ICON_GAP;
            regions.push((right, right + ICON + ICON_GAP, Target::Tray(i)));
            let Some(icon) = &item.icon else {
                let letter: String = item.label.chars().take(1).collect();
                let layout = layout(cr, &letter.to_uppercase(), false);
                let x = right + (ICON + ICON_GAP - f64::from(layout.pixel_size().0)) / 2.0;
                text(cr, &layout, (x, 0.0, h), c.text);
                continue;
            };
            if let Some((_, image)) = icons.iter().find(|(data, _)| Arc::ptr_eq(data, &icon.data)) {
                let _ = cr.save();
                cr.translate(right + ICON_GAP / 2.0, (h - ICON) / 2.0);
                cr.scale(ICON / f64::from(icon.width), ICON / f64::from(icon.height));
                let _ = cr.set_source_surface(image, 0.0, 0.0);
                let _ = cr.paint();
                let _ = cr.restore();
            }
        }
        let blocks = status.as_ref().map_or(&[][..], |s| &s.blocks[..]);
        let laid: Vec<_> = blocks
            .iter()
            .map(|b| {
                let layout = layout(cr, &b.full_text, b.markup.as_deref() == Some("pango"));
                let width = block_width(cr, b, &layout);
                (b, layout, width)
            })
            .collect();
        let last = laid.len().saturating_sub(1);
        let gap = |i: usize, b: &Block| match i < last {
            true => f64::from(b.separator_block_width.unwrap_or(if b.separator == Some(true) { 9 } else { 0 })),
            false => 0.0,
        };
        let total: f64 = laid.iter().enumerate().map(|(i, (b, _, width))| width + gap(i, b)).sum();
        let mut x = right - total;
        for (i, (block, layout, width)) in laid.iter().enumerate() {
            let urgent = |color: Color| block.urgent.then_some(color);
            let parse = |color: &Option<String>| color.as_deref().and_then(Color::parse);
            if let Some(bg) = parse(&block.background).or(urgent(c.urgent)) {
                fill(cr, (x, 0.0, *width, h), bg);
            }
            let own = f64::from(layout.pixel_size().0);
            let offset = match block.align.as_deref() {
                Some("center") => (width - own) / 2.0,
                Some("right") => width - own,
                _ => 0.0,
            };
            let fg = parse(&block.color).or(urgent(c.urgent_text)).unwrap_or(c.text);
            text(cr, layout, (x + offset, 0.0, h), fg);
            regions.push((x, x + width, Target::Block(i)));
            x += width;
            let gap = gap(i, block);
            if gap > 0.0 && block.separator.unwrap_or(block.separator_block_width.is_some()) {
                fill(cr, ((x + gap / 2.0).floor(), 3.0, 1.0, h - 6.0), c.separator);
            }
            x += gap;
        }
    });
    app.bars[index].regions = regions;
}
