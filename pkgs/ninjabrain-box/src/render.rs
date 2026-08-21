//! Turning the bot's state into a laid-out, coloured table.
//!
//! Layout and drawing are separate passes because a layer surface has to name
//! its size before it is given a buffer to draw into.

use crate::config::{AngleCorrection, Color, Config, Coordinates, Palette};
use crate::model::Stronghold;
use crate::text::Text;

/// Where a run of text goes, and in what colour.
pub struct Cell {
    pub text: String,
    pub color: Color,
    pub x: f32,
    pub y: f32,
}

/// A band of background behind some cells.
pub struct Band {
    pub y: f32,
    pub height: f32,
    pub color: Color,
}

pub struct Layout {
    pub width: u32,
    pub height: u32,
    pub bands: Vec<Band>,
    pub cells: Vec<Cell>,
}

/// How the columns of the horizontal table are proportioned, copying the
/// weights the bot's own panel uses so the two look alike.
const COLUMN_WEIGHTS: [f32; 6] = [2.0, 1.0, 1.0, 1.8, 1.6, 1.4];

pub fn layout(state: &Stronghold, config: &Config, text: &mut Text, show_throws: bool) -> Layout {
    let palette = &config.palette;
    let pad = config.window.padding as f32;
    let line = text.line_height();
    let mut builder = Builder {
        cells: Vec::new(),
        bands: Vec::new(),
        y: pad,
        width: 0.0f32,
        pad,
        line,
    };

    match state.placeholder() {
        Some(message) => builder.row(&[(message.to_owned(), palette.base03)], text, None),
        None => predictions(state, config, text, &mut builder),
    }

    if show_throws && !state.eye_throws.is_empty() {
        throws(state, config, text, &mut builder);
    }

    Layout {
        width: (builder.width + pad * 2.0).ceil().max(1.0) as u32,
        height: (builder.y + pad).ceil().max(1.0) as u32,
        bands: builder.bands,
        cells: builder.cells,
    }
}

struct Builder {
    cells: Vec<Cell>,
    bands: Vec<Band>,
    y: f32,
    width: f32,
    pad: f32,
    line: f32,
}

impl Builder {
    /// Lays one line of cells out left to right, each padded to its column.
    fn columns(&mut self, cells: &[(String, Color)], widths: &[f32], text: &Text, band: Option<Color>) {
        if let Some(color) = band {
            self.bands.push(Band {
                y: self.y,
                height: self.line,
                color,
            });
        }
        let mut x = self.pad;
        for ((content, color), width) in cells.iter().zip(widths) {
            // Right-aligned: these are numbers, and the decimal points should
            // line up down the column.
            let offset = (width - text.width(content)).max(0.0);
            self.cells.push(Cell {
                text: content.clone(),
                color: *color,
                x: x + offset,
                y: self.y,
            });
            x += width + text.advance();
        }
        self.width = self.width.max(x - text.advance() - self.pad);
        self.y += self.line;
    }

    /// A line that is one run of text.
    fn row(&mut self, cells: &[(String, Color)], text: &Text, band: Option<Color>) {
        let widths: Vec<f32> = cells.iter().map(|(c, _)| text.width(c)).collect();
        self.columns(cells, &widths, text, band);
    }

    fn gap(&mut self) {
        self.y += self.line * 0.35;
    }
}

/// Column widths that fit every row, respecting the panel's proportions.
fn widths(rows: &[Vec<(String, Color)>], text: &Text, weights: &[f32]) -> Vec<f32> {
    let count = rows.iter().map(Vec::len).max().unwrap_or(0);
    (0..count)
        .map(|column| {
            let content = rows
                .iter()
                .filter_map(|row| row.get(column))
                .map(|(cell, _)| text.width(cell))
                .fold(0.0f32, f32::max);
            let floor = weights.get(column).copied().unwrap_or(1.0) * text.advance() * 2.0;
            content.max(floor)
        })
        .collect()
}

fn predictions(state: &Stronghold, config: &Config, text: &mut Text, builder: &mut Builder) {
    let palette = &config.palette;
    let header = vec![
        (location_header(config), palette.base03),
        ("%".to_owned(), palette.base03),
        ("Dist.".to_owned(), palette.base03),
        ("Nether".to_owned(), palette.base03),
        ("Angle".to_owned(), palette.base03),
        ("Turn".to_owned(), palette.base03),
    ];

    let mut rows = vec![header];
    for prediction in state.predictions.iter().take(config.window.predictions) {
        let player = &state.player_position;
        rows.push(vec![
            (
                format_pair(location(prediction, config)),
                coordinate_color(location(prediction, config), palette, config),
            ),
            (
                format!("{:.1}", prediction.certainty * 100.0),
                certainty_color(prediction.certainty, palette),
            ),
            (
                format!("{:.0}", prediction.distance(player)),
                palette.base0d,
            ),
            (
                format_pair(prediction.nether()),
                palette.base0c,
            ),
            (
                prediction
                    .travel_angle(player)
                    .map_or_else(|| "-".to_owned(), |angle| format!("{angle:.1}")),
                palette.base0e,
            ),
            turn(prediction, player, palette),
        ]);
    }

    let widths = widths(&rows, text, &COLUMN_WEIGHTS);
    let Some((header, body)) = rows.split_first() else {
        return;
    };
    builder.columns(header, &widths, text, Some(palette.base01));
    for (index, row) in body.iter().enumerate() {
        let band = (index % 2 == 1).then_some(palette.base02);
        builder.columns(row, &widths, text, band);
    }
}

fn throws(state: &Stronghold, config: &Config, text: &mut Text, builder: &mut Builder) {
    let palette = &config.palette;
    builder.gap();
    let correction_header = match config.window.angle_correction {
        AngleCorrection::Increments => "Steps",
        AngleCorrection::Degrees => "Corr.",
    };
    let mut rows = vec![vec![
        ("x".to_owned(), palette.base03),
        ("z".to_owned(), palette.base03),
        ("Angle".to_owned(), palette.base03),
        (correction_header.to_owned(), palette.base03),
        ("Error".to_owned(), palette.base03),
    ]];
    for throw in &state.eye_throws {
        rows.push(vec![
            (format!("{:.1}", throw.x), palette.base05),
            (format!("{:.1}", throw.z), palette.base05),
            (format!("{:.2}", throw.angle), palette.base0e),
            correction(throw, config, palette),
            (
                format!("{:.3}", throw.error),
                error_color(throw.error, palette),
            ),
        ]);
    }
    let widths = widths(&rows, text, &[1.5, 1.5, 1.5, 1.2, 1.5]);
    let Some((header, body)) = rows.split_first() else {
        return;
    };
    builder.columns(header, &widths, text, Some(palette.base01));
    for row in body {
        builder.columns(row, &widths, text, None);
    }
}

fn location(prediction: &crate::model::Prediction, config: &Config) -> (i32, i32) {
    match config.window.coordinates {
        Coordinates::Chunk => prediction.chunk(),
        Coordinates::Block => prediction.block(),
    }
}

fn location_header(config: &Config) -> String {
    match config.window.coordinates {
        Coordinates::Chunk => "Chunk".to_owned(),
        Coordinates::Block => "Location".to_owned(),
    }
}

fn format_pair((x, z): (i32, i32)) -> String {
    format!("{x}, {z}")
}

fn coordinate_color((x, z): (i32, i32), palette: &Palette, config: &Config) -> Color {
    if config.window.color_negative_coordinates && (x < 0 || z < 0) {
        palette.base08
    } else {
        palette.base05
    }
}

/// Red through to green as the bot grows more sure.
fn certainty_color(certainty: f64, palette: &Palette) -> Color {
    let t = certainty as f32;
    if t < 0.5 {
        palette.base08.mix(palette.base09, t * 2.0)
    } else if t < 0.9 {
        palette.base09.mix(palette.base0a, (t - 0.5) / 0.4)
    } else {
        palette.base0a.mix(palette.base0b, (t - 0.9) / 0.1)
    }
}

/// How far the player still has to turn, and how urgent that is: green once
/// they are pointing at it, red while they are not.
fn turn(
    prediction: &crate::model::Prediction,
    player: &crate::model::Player,
    palette: &Palette,
) -> (String, Color) {
    match prediction.travel_angle_delta(player) {
        None => (String::new(), palette.base03),
        Some(delta) => {
            let t = (delta.abs() as f32 / 30.0).clamp(0.0, 1.0);
            (format!("{delta:+.1}"), palette.base0b.mix(palette.base08, t))
        }
    }
}

/// How far a throw's angle was nudged, and which way.
///
/// Signed and coloured, because the sign is the whole point -- it says which
/// way to correct next time. Nothing is drawn when there was no correction,
/// which is what the bot does too.
fn correction(throw: &crate::model::Throw, config: &Config, palette: &Palette) -> (String, Color) {
    let (sign, text) = match config.window.angle_correction {
        AngleCorrection::Increments => (
            throw.correction_increments.signum(),
            format!("{:+}", throw.correction_increments),
        ),
        AngleCorrection::Degrees => (
            // The bot treats anything under a ten-millionth as none at all.
            if throw.correction.abs() < 1e-7 {
                0
            } else if throw.correction > 0.0 {
                1
            } else {
                -1
            },
            format!("{:+.3}", throw.correction),
        ),
    };
    match sign {
        0 => (String::new(), palette.base03),
        1 => (text, palette.base0b),
        _ => (text, palette.base08),
    }
}

/// A throw's angle error, green while it is small.
fn error_color(error: f64, palette: &Palette) -> Color {
    let t = (error.abs() as f32 / 0.05).clamp(0.0, 1.0);
    palette.base0b.mix(palette.base08, t)
}

/// A premultiplied ARGB8888 buffer, as `wl_shm` wants one.
pub struct Canvas<'a> {
    pub pixels: &'a mut [u8],
    pub width: i32,
    pub height: i32,
}

impl Canvas<'_> {
    pub fn fill(&mut self, color: Color) {
        self.rect(0, 0, self.width, self.height, color);
    }

    pub fn rect(&mut self, x: i32, y: i32, width: i32, height: i32, color: Color) {
        for row in y.max(0)..(y + height).min(self.height) {
            for column in x.max(0)..(x + width).min(self.width) {
                self.blend(column, row, color, 1.0);
            }
        }
    }

    /// Source-over with `coverage` scaling the source alpha.
    pub fn blend(&mut self, x: i32, y: i32, color: Color, coverage: f32) {
        if x < 0 || y < 0 || x >= self.width || y >= self.height || coverage <= 0.0 {
            return;
        }
        let alpha = color.3 as f32 / 255.0 * coverage.min(1.0);
        if alpha <= 0.0 {
            return;
        }
        let offset = ((y * self.width + x) * 4) as usize;
        let pixel = &mut self.pixels[offset..offset + 4];
        // Little-endian ARGB8888 is B, G, R, A in memory, premultiplied.
        for (index, channel) in [color.2, color.1, color.0].into_iter().enumerate() {
            let source = channel as f32 / 255.0 * alpha;
            let destination = pixel[index] as f32 / 255.0;
            pixel[index] = ((source + destination * (1.0 - alpha)) * 255.0).round() as u8;
        }
        let destination = pixel[3] as f32 / 255.0;
        pixel[3] = ((alpha + destination * (1.0 - alpha)) * 255.0).round() as u8;
    }
}

pub fn draw(layout: &Layout, canvas: &mut Canvas, text: &mut Text, config: &Config) {
    canvas.fill(config.palette.base00.with_alpha(config.window.opacity));
    for band in &layout.bands {
        canvas.rect(
            0,
            band.y.round() as i32,
            canvas.width,
            band.height.ceil() as i32,
            band.color.with_alpha(config.window.opacity),
        );
    }
    for cell in &layout.cells {
        let mut pen = cell.x;
        let advance = text.advance();
        let baseline = cell.y + text.ascent();
        for character in cell.text.chars() {
            let glyph = text.glyph(character);
            let metrics = glyph.0;
            let bitmap: &[u8] = &glyph.1;
            let (width, height) = (metrics.width as i32, metrics.height as i32);
            let left = pen.round() as i32 + metrics.xmin;
            let top = baseline.round() as i32 - metrics.height as i32 - metrics.ymin;
            for row in 0..height {
                for column in 0..width {
                    let coverage = bitmap[(row * width + column) as usize] as f32 / 255.0;
                    canvas.blend(left + column, top + row, cell.color, coverage);
                }
            }
            pen += advance;
        }
    }
}
