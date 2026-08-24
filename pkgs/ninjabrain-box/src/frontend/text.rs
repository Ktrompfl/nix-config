//! Glyphs.
//!
//! The panel is a table of numbers, so it is laid out on a fixed advance and
//! drawn from a rasterizer cache. That keeps columns lined up without a
//! shaping engine, and keeps the dependency list short.

use anyhow::{Context, Result, anyhow};
use std::collections::HashMap;
use std::path::PathBuf;

use fontdue::{Font, FontSettings};

use crate::config::Config;

pub struct Text {
    font: Font,
    size: f32,
    advance: f32,
    ascent: f32,
    line_height: f32,
    cache: HashMap<char, (fontdue::Metrics, Vec<u8>)>,
}

impl Text {
    /// The font the table is set in, at the size the configuration asks for.
    pub fn load(config: &Config) -> Result<Text> {
        let path = config
            .window
            .font
            .clone()
            .or_else(|| std::env::var_os("NINJABRAIN_BOX_FONT").map(PathBuf::from))
            .context("no font configured, and NINJABRAIN_BOX_FONT is unset")?;
        let bytes = std::fs::read(&path)
            .with_context(|| format!("cannot read {}", path.display()))?;
        Text::new(&bytes, config.window.font_size)
    }

    pub fn new(bytes: &[u8], size: f32) -> Result<Text> {
        let font = Font::from_bytes(bytes, FontSettings::default()).map_err(|e| anyhow!("{e}"))?;
        let metrics = font.horizontal_line_metrics(size).context("font has no horizontal metrics")?;
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
