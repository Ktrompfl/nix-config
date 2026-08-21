//! Glyphs.
//!
//! The panel is a table of numbers, so it is laid out on a fixed advance and
//! drawn from a rasterizer cache. That keeps columns lined up without a
//! shaping engine, and keeps the dependency list short.

use std::collections::HashMap;

use fontdue::{Font, FontSettings};

pub struct Text {
    font: Font,
    size: f32,
    advance: f32,
    ascent: f32,
    line_height: f32,
    cache: HashMap<char, (fontdue::Metrics, Vec<u8>)>,
}

impl Text {
    pub fn new(bytes: &[u8], size: f32) -> Result<Text, String> {
        let font = Font::from_bytes(bytes, FontSettings::default())?;
        let metrics = font.horizontal_line_metrics(size).ok_or("font has no horizontal metrics")?;
        // Every digit shares an advance in any sane font; '0' stands for them.
        let advance = font.metrics('0', size).advance_width;
        Ok(Text {
            size,
            advance,
            ascent: metrics.ascent,
            line_height: (metrics.ascent - metrics.descent + metrics.line_gap).ceil(),
            font,
            cache: HashMap::new(),
        })
    }

    pub fn advance(&self) -> f32 {
        self.advance
    }

    pub fn line_height(&self) -> f32 {
        self.line_height
    }

    pub fn ascent(&self) -> f32 {
        self.ascent
    }

    /// The width one string will take at the fixed advance.
    pub fn width(&self, text: &str) -> f32 {
        text.chars().count() as f32 * self.advance
    }

    /// The coverage bitmap for one character.
    pub fn glyph(&mut self, character: char) -> &(fontdue::Metrics, Vec<u8>) {
        let size = self.size;
        self.cache
            .entry(character)
            .or_insert_with(|| self.font.rasterize(character, size))
    }
}
