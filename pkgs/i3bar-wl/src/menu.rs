use crate::{App, config::Colors, draw, tray::Entry};
use smithay_client_toolkit::{
    reexports::client::{
        QueueHandle,
        protocol::{wl_seat::WlSeat, wl_surface::WlSurface},
    },
    shell::xdg::{
        XdgPositioner, XdgShell,
        popup::{Popup, PopupConfigure},
    },
    shm::slot::SlotPool,
};
use wayland_protocols::{
    wp::viewporter::client::{wp_viewport::WpViewport, wp_viewporter::WpViewporter},
    xdg::shell::client::{xdg_positioner, xdg_surface::XdgSurface},
};

const ROW: f64 = 22.0;
const SEPARATOR: f64 = 7.0;
const PAD: f64 = 10.0;
const MARK: f64 = 16.0;

#[derive(Clone, Copy)]
pub enum Place {
    Up,
    Down,
    Side,
}

pub struct Menu {
    pub item: String,
    pub scale: u32,
    pub levels: Vec<Level>,
    pub pending: Option<(i32, usize, usize)>,
}

pub struct Level {
    popup: Popup,
    viewport: Option<WpViewport>,
    entries: Vec<Entry>,
    size: (u32, u32),
    configured: bool,
    hover: Option<usize>,
}

pub struct Open<'a> {
    pub qh: &'a QueueHandle<App>,
    pub xdg: &'a XdgShell,
    pub viewporter: Option<&'a WpViewporter>,
    pub surface: WlSurface,
}

fn rows(entries: &[Entry]) -> Vec<(f64, f64)> {
    let mut y = 1.0;
    entries
        .iter()
        .map(|e| {
            let h = if e.separator { SEPARATOR } else { ROW };
            y += h;
            (y - h, h)
        })
        .collect()
}

fn size(entries: &[Entry]) -> (u32, u32) {
    let marks = entries.iter().any(|e| e.toggle.is_some());
    let arrows = entries.iter().any(|e| e.submenu);
    let label = entries.iter().map(|e| draw::measure(&e.label)).fold(0.0, f64::max);
    let width = label + 2.0 * PAD + if marks { MARK } else { 0.0 } + if arrows { MARK } else { 0.0 };
    let height = rows(entries).last().map_or(2.0, |(y, h)| y + h + 1.0);
    (width.max(120.0).ceil() as u32, height.ceil() as u32)
}

impl Level {
    pub fn open(
        open: Open,
        parent: Option<&XdgSurface>,
        entries: Vec<Entry>,
        anchor: (i32, i32, i32, i32),
        place: Place,
    ) -> Option<Self> {
        let size = size(&entries);
        let positioner = XdgPositioner::new(open.xdg).ok()?;
        positioner.set_size(size.0 as i32, size.1 as i32);
        positioner.set_anchor_rect(anchor.0, anchor.1, anchor.2.max(1), anchor.3.max(1));
        use xdg_positioner::{Anchor, ConstraintAdjustment, Gravity};
        let (a, g) = match place {
            Place::Up => (Anchor::TopLeft, Gravity::TopRight),
            Place::Down => (Anchor::BottomLeft, Gravity::BottomRight),
            Place::Side => (Anchor::TopRight, Gravity::BottomRight),
        };
        positioner.set_anchor(a);
        positioner.set_gravity(g);
        use ConstraintAdjustment as C;
        positioner.set_constraint_adjustment(C::SlideX | C::SlideY | C::FlipX | C::FlipY);
        let popup = Popup::from_surface(parent, &positioner, open.qh, open.surface.clone(), open.xdg).ok()?;
        let viewport = open.viewporter.map(|v| v.get_viewport(&open.surface, open.qh, crate::Silent));
        Some(Self { popup, viewport, entries, size, configured: false, hover: None })
    }

    pub fn popup(&self) -> &smithay_client_toolkit::reexports::protocols::xdg::shell::client::xdg_popup::XdgPopup {
        self.popup.xdg_popup()
    }

    pub fn grab(&self, seat: &WlSeat, serial: u32) {
        self.popup.xdg_popup().grab(seat, serial);
    }

    pub fn commit(&self) {
        self.popup.wl_surface().commit();
    }

    pub fn surface(&self) -> &WlSurface {
        self.popup.wl_surface()
    }

    pub fn xdg_surface(&self) -> &XdgSurface {
        self.popup.xdg_surface()
    }

    pub fn is(&self, popup: &Popup) -> bool {
        self.popup.wl_surface() == popup.wl_surface()
    }

    pub fn configure(&mut self, config: &PopupConfigure) {
        if config.width > 0 && config.height > 0 {
            self.size = (config.width as u32, config.height as u32);
        }
        self.configured = true;
    }

    pub fn at(&self, y: f64) -> Option<usize> {
        rows(&self.entries).iter().position(|(top, h)| (*top..top + h).contains(&y))
    }

    pub fn entry(&self, index: usize) -> Option<&Entry> {
        self.entries.get(index)
    }

    pub fn row(&self, index: usize) -> (i32, i32, i32, i32) {
        let (y, h) = rows(&self.entries)[index];
        (0, y as i32, self.size.0 as i32, h as i32)
    }

    pub fn hover(&mut self, index: Option<usize>) -> bool {
        let index = index.filter(|&i| self.entries[i].enabled && !self.entries[i].separator);
        std::mem::replace(&mut self.hover, index) != index
    }

    pub fn close(self) {
        self.viewport.iter().for_each(|v| v.destroy());
    }

    pub fn draw(&self, pool: &mut SlotPool, scale: u32, c: &Colors) {
        if !self.configured {
            return;
        }
        let (w, h) = self.size;
        let marks = self.entries.iter().any(|e| e.toggle.is_some());
        let target = (self.popup.wl_surface(), self.viewport.as_ref());
        draw::present(pool, target, (w, h, scale), |cr| {
            let (w, h) = (f64::from(w), f64::from(h));
            draw::fill(cr, (0.0, 0.0, w, h), c.menu_border);
            draw::fill(cr, (1.0, 1.0, w - 2.0, h - 2.0), c.menu);
            for (i, (entry, (y, rh))) in self.entries.iter().zip(rows(&self.entries)).enumerate() {
                if entry.separator {
                    draw::fill(cr, (PAD, y + rh / 2.0, w - 2.0 * PAD, 1.0), c.menu_border);
                    continue;
                }
                let fg = match () {
                    _ if !entry.enabled => c.menu_disabled,
                    _ if self.hover == Some(i) => {
                        draw::fill(cr, (1.0, y, w - 2.0, rh), c.active);
                        c.active_text
                    }
                    _ => c.text,
                };
                let mut x = PAD;
                if marks {
                    let mark = match entry.toggle {
                        Some((false, true)) => "✓",
                        Some((true, true)) => "●",
                        Some((true, false)) => "○",
                        _ => "",
                    };
                    draw::text(cr, &draw::layout(cr, mark, false), (x, y, rh), fg);
                    x += MARK;
                }
                draw::text(cr, &draw::layout(cr, &entry.label, false), (x, y, rh), fg);
                if entry.submenu {
                    draw::text(cr, &draw::layout(cr, "›", false), (w - PAD - 6.0, y, rh), fg);
                }
            }
        });
    }
}
