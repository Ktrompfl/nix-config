#![deny(unsafe_code)]

mod vulkan;

pub use geom::{Color, Rect};
pub use vulkan::{Destination, Dmabuf, Frame, Op, Plane, Renderer, TexId, attach_fence};

pub trait Source {
    fn size(&self) -> (i32, i32);
    fn opaque(&self) -> bool;
    fn read_row(&self, x: i32, y: i32, out: &mut [u32]);
}

pub struct Pixels {
    pub data: Vec<u32>,
    pub width: i32,
    pub height: i32,
}

impl Source for Pixels {
    fn size(&self) -> (i32, i32) {
        (self.width, self.height)
    }

    fn opaque(&self) -> bool {
        false
    }

    fn read_row(&self, x: i32, y: i32, out: &mut [u32]) {
        let start = (y * self.width + x) as usize;
        match self.data.get(start..start + out.len()) {
            Some(row) if x >= 0 && y >= 0 => out.copy_from_slice(row),
            _ => out.fill(0),
        }
    }
}

pub fn text(text: &str, font: &str, fg: Color, bg: Color, (width, height): (i32, i32), scale: f64) -> Option<Pixels> {
    let mut surface = cairo::ImageSurface::create(cairo::Format::ARgb32, width, height).ok()?;
    {
        let cr = cairo::Context::new(&surface).ok()?;
        source(&cr, bg);
        cr.paint().ok()?;
        let layout = pangocairo::functions::create_layout(&cr);
        let mut description = pango::FontDescription::from_string(font);
        description.set_size((f64::from(description.size()) * scale) as i32);
        layout.set_font_description(Some(&description));
        layout.set_text(text);
        layout.set_ellipsize(pango::EllipsizeMode::End);
        let padding = (4.0 * scale) as i32;
        layout.set_width((width - 2 * padding).max(0) * pango::SCALE);
        let (_, text_height) = layout.pixel_size();
        source(&cr, fg);
        cr.move_to(f64::from(padding), f64::from((height - text_height) / 2));
        pangocairo::functions::show_layout(&cr, &layout);
    }
    surface.flush();
    let stride = surface.stride() as usize;
    let bytes = surface.data().ok()?;
    let rows = bytes.chunks(stride).take(height as usize);
    let data = rows
        .flat_map(|row| row[..width as usize * 4].as_chunks::<4>().0.iter().map(|&p| u32::from_ne_bytes(p)))
        .collect();
    Some(Pixels { data, width, height })
}

fn source(cr: &cairo::Context, color: Color) {
    let channel = |shift: u32| f64::from((color >> shift) & 0xff) / 255.0;
    cr.set_source_rgba(channel(16), channel(8), channel(0), channel(24));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_fills_and_textures() {
        let mut renderer = match Renderer::new() {
            Ok(renderer) => renderer,
            Err(e) => return eprintln!("skipping, no GPU: {e}"),
        };
        let red = Pixels { data: vec![0xffff0000; 16], width: 4, height: 4 };
        let tex = renderer.upload(None, &red).unwrap();
        let (source, dst, clip) = ([0.0, 0.0, 4.0, 4.0], Rect::new(4, 4, 4, 4), Rect::new(0, 0, 8, 8));
        let ops = [Op::Fill(clip, 0xff00ff00), Op::Texture { tex, source, transform: 0, dst, clip, alpha: 1.0 }];
        let mut out = vec![0; 64];
        renderer.render(&ops, Frame { size: (8, 8), transform: 0 }, Destination::Memory(&mut out), None).unwrap();
        assert_eq!((out[0], out[63]), (0xff00ff00, 0xffff0000));
        assert!(!renderer.formats().is_empty());
    }

    #[test]
    fn uploads_regions_and_premultiplies_fills() {
        let Ok(mut renderer) = Renderer::new() else { return };
        let red = Pixels { data: vec![0xffff0000; 16], width: 4, height: 4 };
        let tex = renderer.upload(None, &red).unwrap();
        let (frame, full) = (Frame { size: (4, 4), transform: 0 }, Rect::new(0, 0, 4, 4));
        let draw = [Op::Texture { tex, source: [0.0, 0.0, 4.0, 4.0], transform: 0, dst: full, clip: full, alpha: 1.0 }];
        let mut out = vec![0; 16];
        renderer.render(&draw, frame, Destination::Memory(&mut out), None).unwrap();
        let green = Pixels { data: vec![0xff00ff00; 16], width: 4, height: 4 };
        assert_eq!(renderer.upload_region(tex, &green, Rect::new(0, 0, 2, 2)).unwrap(), tex);
        renderer.render(&draw, frame, Destination::Memory(&mut out), None).unwrap();
        assert_eq!((out[0], out[5], out[15]), (0xff00ff00, 0xff00ff00, 0xffff0000));
        renderer.render(&[Op::Fill(full, 0x80ffffff)], frame, Destination::Memory(&mut out), None).unwrap();
        assert!((0x7f..=0x81).contains(&(out[0] & 0xff)) && out[0] >> 24 == 0xff, "{:x}", out[0]);
    }

    #[test]
    fn transformed_textures_are_sampled_with_the_inverse_transform() {
        let Ok(mut renderer) = Renderer::new() else { return };
        let tex = renderer.upload(None, &Pixels { data: vec![0xffff0000, 0xff00ff00], width: 2, height: 1 }).unwrap();
        let (frame, dst) = (Frame { size: (1, 2), transform: 0 }, Rect::new(0, 0, 1, 2));
        let mut out = vec![0; 2];
        for (transform, expected) in [(1, [0xffff0000, 0xff00ff00]), (3, [0xff00ff00, 0xffff0000])] {
            let op = Op::Texture { tex, source: [0.0, 0.0, 2.0, 1.0], transform, dst, clip: dst, alpha: 1.0 };
            renderer.render(&[op], frame, Destination::Memory(&mut out), None).unwrap();
            assert_eq!(out, expected);
        }
    }

    #[test]
    fn rotated_frames_map_logical_space_onto_the_framebuffer() {
        let Ok(mut renderer) = Renderer::new() else { return };
        let frame = Frame { size: (4, 8), transform: 1 };
        assert_eq!(frame.logical_size(), (8, 4));
        let ops = [Op::Fill(Rect::new(0, 0, 8, 4), 0xff00ff00), Op::Fill(Rect::new(0, 0, 4, 4), 0xffff0000)];
        let mut out = vec![0; 32];
        renderer.render(&ops, frame, Destination::Memory(&mut out), None).unwrap();
        assert_eq!((out[0], out[31]), (0xff00ff00, 0xffff0000));
    }
}
