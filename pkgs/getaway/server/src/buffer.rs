use crate::{Key, shm::ShmBuffer};
use gfx::{Dmabuf, Pixels, Renderer, Source, TexId};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::atomic::{AtomicU64, Ordering::Relaxed},
};
use wm::Rect;

pub type Garbage = Rc<RefCell<Vec<(u64, Option<TexId>)>>>;

pub enum Data {
    Shm(ShmBuffer),
    Dmabuf(Dmabuf),
    Pixel(Pixels),
}

static SERIALS: AtomicU64 = AtomicU64::new(0);

pub struct Buffer {
    pub id: u32,
    pub serial: u64,
    data: Data,
    texture: Cell<Option<TexId>>,
    damage: Cell<Rect>,
    last: Cell<Option<(Key, u64)>>,
    sampled: Cell<u64>,
    garbage: Garbage,
}

impl Buffer {
    pub fn new(id: u32, data: Data, texture: Option<TexId>, garbage: Garbage) -> Self {
        let serial = SERIALS.fetch_add(1, Relaxed) + 1;
        let damage = Cell::new(Rect::default());
        let (texture, last, sampled) = (Cell::new(texture), Cell::new(None), Cell::new(0));
        Self { id, serial, data, texture, damage, last, sampled, garbage }
    }

    pub fn size(&self) -> (i32, i32) {
        match &self.data {
            Data::Shm(shm) => shm.size(),
            Data::Dmabuf(dmabuf) => dmabuf.size,
            Data::Pixel(_) => (1, 1),
        }
    }

    pub fn shm(&self) -> Option<&ShmBuffer> {
        match &self.data {
            Data::Shm(shm) => Some(shm),
            Data::Dmabuf(_) | Data::Pixel(_) => None,
        }
    }

    pub fn dmabuf(&self) -> Option<&Dmabuf> {
        match &self.data {
            Data::Dmabuf(dmabuf) => Some(dmabuf),
            _ => None,
        }
    }

    pub fn faulted(&self) -> bool {
        self.shm().is_some_and(|shm| shm.faulted())
    }

    pub fn damage(&self, region: Rect) {
        self.damage.set(self.damage.get().union(region));
    }

    pub fn last(&self) -> Option<(Key, u64)> {
        self.last.get()
    }

    pub fn set_last(&self, key: Key, commit: u64) {
        self.last.set(Some((key, commit)));
    }

    pub fn busy(&self, renderer: &Renderer) -> bool {
        self.sampled.get() > renderer.completed()
    }

    pub fn texture(&self, renderer: &mut Renderer) -> Option<TexId> {
        self.sampled.set(renderer.submitted() + 1);
        let damage = self.damage.take();
        let texture = match (&self.data, self.texture.get()) {
            (Data::Dmabuf(_) | Data::Pixel(_), Some(texture)) => return Some(texture),
            (Data::Pixel(pixel), None) => renderer.upload(None, pixel),
            (Data::Shm(_), Some(texture)) if damage.is_empty() => return Some(texture),
            (Data::Dmabuf(dmabuf), None) => renderer.import(dmabuf, false),
            (Data::Shm(shm), Some(existing)) => renderer.upload_region(existing, shm, damage),
            (Data::Shm(shm), None) => renderer.upload(None, shm),
        };
        let texture = texture
            .inspect_err(|e| {
                log::warn!("cannot create texture: {e}");
                self.damage(damage);
            })
            .ok()?;
        self.texture.set(Some(texture));
        Some(texture)
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        self.garbage.borrow_mut().push((self.serial, self.texture.get()));
    }
}
