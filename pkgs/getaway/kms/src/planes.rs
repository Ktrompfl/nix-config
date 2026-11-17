use crate::{Card, Result, property_value};
use drm::{
    buffer::{self, PlanarBuffer},
    control::{Device as _, FbCmd2Flags, framebuffer, plane},
};
use gbm::{BufferObject, BufferObjectFlags, Format, Modifier};
use std::{collections::HashMap, os::fd::AsFd};

pub struct ClientFb {
    pub framebuffer: framebuffer::Handle,
    handles: Vec<buffer::Handle>,
}

struct Planar<'a> {
    dmabuf: &'a gfx::Dmabuf,
    format: Format,
    handles: [Option<buffer::Handle>; 4],
}

impl PlanarBuffer for Planar<'_> {
    fn size(&self) -> (u32, u32) {
        (self.dmabuf.size.0 as u32, self.dmabuf.size.1 as u32)
    }

    fn format(&self) -> Format {
        self.format
    }

    fn modifier(&self) -> Option<Modifier> {
        Some(Modifier::from(self.dmabuf.modifier))
    }

    fn pitches(&self) -> [u32; 4] {
        let mut pitches = [0; 4];
        self.dmabuf.planes.iter().zip(&mut pitches).for_each(|(p, out)| *out = p.stride);
        pitches
    }

    fn handles(&self) -> [Option<buffer::Handle>; 4] {
        self.handles
    }

    fn offsets(&self) -> [u32; 4] {
        let mut offsets = [0; 4];
        self.dmabuf.planes.iter().zip(&mut offsets).for_each(|(p, out)| *out = p.offset);
        offsets
    }
}

pub type Handles = HashMap<buffer::Handle, usize>;

pub fn import(card: &Card, dmabuf: &gfx::Dmabuf, refs: &mut Handles) -> Result<ClientFb> {
    let format = Format::try_from(dmabuf.format).map_err(|_| "unknown format")?;
    let mut handles = vec![];
    for plane in dmabuf.planes.iter().take(4) {
        match card.prime_fd_to_buffer(plane.fd.as_fd()) {
            Ok(handle) => {
                *refs.entry(handle).or_default() += 1;
                handles.push(handle);
            }
            Err(e) => return Err(close(card, handles, refs, e.into())),
        }
    }
    let mut array = [None; 4];
    handles.iter().zip(&mut array).for_each(|(&h, out)| *out = Some(h));
    match card.add_planar_framebuffer(&Planar { dmabuf, format, handles: array }, FbCmd2Flags::MODIFIERS) {
        Ok(framebuffer) => Ok(ClientFb { framebuffer, handles }),
        Err(e) => Err(close(card, handles, refs, e.into())),
    }
}

fn close(
    card: &Card,
    handles: Vec<buffer::Handle>,
    refs: &mut Handles,
    error: Box<dyn std::error::Error>,
) -> Box<dyn std::error::Error> {
    for handle in handles {
        let Some(count) = refs.get_mut(&handle) else { continue };
        *count -= 1;
        if *count == 0 {
            refs.remove(&handle);
            let _ = card.close_buffer(handle);
        }
    }
    error
}

pub fn release(card: &Card, fb: ClientFb, refs: &mut Handles) {
    let _ = card.destroy_framebuffer(fb.framebuffer);
    close(card, fb.handles, refs, "released".into());
}

pub struct CursorPlane {
    pub plane: plane::Handle,
    pub size: (u32, u32),
    buffers: Vec<(BufferObject<()>, framebuffer::Handle)>,
    written: Option<(u64, usize)>,
    committed: Option<usize>,
    bytes: Vec<u8>,
}

impl CursorPlane {
    pub fn new(card: &Card, gbm: &gbm::Device<Card>, plane: plane::Handle, size: (u32, u32)) -> Result<Self> {
        let flags = BufferObjectFlags::CURSOR | BufferObjectFlags::WRITE | BufferObjectFlags::LINEAR;
        let buffers = (0..2)
            .map(|_| {
                let bo = gbm.create_buffer_object::<()>(size.0, size.1, Format::Argb8888, flags)?;
                let framebuffer = card.add_planar_framebuffer(&bo, FbCmd2Flags::MODIFIERS)?;
                Ok((bo, framebuffer))
            })
            .collect::<Result<_>>()?;
        let bytes = vec![0; size.0 as usize * size.1 as usize * 4];
        Ok(Self { plane, size, buffers, written: None, committed: None, bytes })
    }

    pub fn framebuffer(&mut self, key: u64, pixels: &[u32], (w, h): (i32, i32)) -> Result<framebuffer::Handle> {
        if let Some((_, index)) = self.written.filter(|s| s.0 == key) {
            return Ok(self.buffers[index].1);
        }
        let (cw, ch) = (self.size.0 as usize, self.size.1 as usize);
        let (w, h) = (w as usize, h as usize);
        if w > cw || h > ch || pixels.len() < w * h {
            return Err("cursor does not fit the plane".into());
        }
        self.bytes.fill(0);
        for (row, src) in self.bytes.chunks_mut(cw * 4).zip(pixels.chunks(w)).take(h) {
            row[..w * 4].iter_mut().zip(src.iter().flat_map(|p| p.to_le_bytes())).for_each(|(d, s)| *d = s);
        }
        let index = self.committed.map_or(0, |c| 1 - c);
        self.written = None;
        self.buffers[index].0.write(&self.bytes)?;
        self.written = Some((key, index));
        Ok(self.buffers[index].1)
    }

    pub fn committed(&mut self, shown: bool) {
        self.committed = self.written.map(|w| w.1).filter(|_| shown);
    }

    pub fn destroy(self, card: &Card) {
        self.buffers.into_iter().for_each(|(_, fb)| drop(card.destroy_framebuffer(fb)));
    }
}

pub fn formats(card: &Card, plane: plane::Handle) -> Vec<(u32, u64)> {
    let Some(blob) = property_value(card, plane, "IN_FORMATS") else { return vec![] };
    let Ok(data) = card.get_property_blob(blob) else { return vec![] };
    let word = |at: usize| data.get(at..at + 4).map_or(0, |b| u32::from_ne_bytes(b.try_into().unwrap()));
    let (count_formats, formats_offset) = (word(8) as usize, word(12) as usize);
    let (count_modifiers, modifiers_offset) = (word(16) as usize, word(20) as usize);
    let formats: Vec<u32> = (0..count_formats).map(|i| word(formats_offset + i * 4)).collect();
    let mut pairs = vec![];
    for i in 0..count_modifiers {
        let at = modifiers_offset + i * 24;
        let (Some(mask), Some(modifier)) = (data.get(at..at + 8), data.get(at + 16..at + 24)) else { continue };
        let mask = u64::from_ne_bytes(mask.try_into().unwrap());
        let (offset, modifier) = (word(at + 8) as usize, u64::from_ne_bytes(modifier.try_into().unwrap()));
        for bit in (0..64).filter(|b| mask & (1 << b) != 0) {
            pairs.extend(formats.get(offset + bit).map(|&f| (f, modifier)));
        }
    }
    pairs
}
