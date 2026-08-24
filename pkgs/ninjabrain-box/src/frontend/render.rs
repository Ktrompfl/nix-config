//! Turning a [`Solution`] into a laid-out, coloured panel.
//!
//! Layout and drawing are separate passes because a layer surface has to name
//! its size before it is given a buffer to draw into. Layout is also what hit
//! testing runs against, so the buttons come out of the same pass as the text.
//!
//! Nothing here knows which calculator produced the solution. Messages arrive
//! as text, a boat reading is present only when it means something, and
//! whether a button can be pressed is answered by [`Options::permits`] rather
//! than worked out from any calculator's rules.
//!
//! The buttons are always drawn and cost no width. The panel is as wide as the
//! prediction table makes it, and everything else -- the throw table, the
//! controls along the top -- is narrower, so the buttons go in the slack at
//! the right-hand edge. Finding that edge takes two passes: one to measure,
//! one to place.

use crate::action::{Index, Request};
use crate::config::{AngleCorrection, Color, Config, Coordinates, Palette};
use crate::model::{Blind, Prediction, Severity, Solution, ThrowReport};

use tiny_skia::{Paint, Pixmap, PixmapMut, PixmapPaint, Rect, Shader, Transform};

use super::text::Text;

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

/// Something to press, and what it asks for.
pub struct Button {
    pub label: String,
    pub request: Request,
    /// A button for something that cannot be done here is drawn, but muted and
    /// deaf -- so the panel does not silently rearrange itself between modes.
    pub enabled: bool,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Button {
    pub fn covers(&self, x: f64, y: f64) -> bool {
        x >= self.x as f64
            && y >= self.y as f64
            && x < (self.x + self.width) as f64
            && y < (self.y + self.height) as f64
    }
}

#[derive(Default)]
pub struct Layout {
    pub width: u32,
    pub height: u32,
    pub bands: Vec<Band>,
    pub cells: Vec<Cell>,
    pub buttons: Vec<Button>,
}

/// One thing in a row.
enum Item {
    Text(String, Color),
    Press(String, Request, bool),
}

fn cell(content: impl Into<String>, color: Color) -> Item {
    Item::Text(content.into(), color)
}

/// How the columns of the prediction table are proportioned, copying the
/// weights the bot's own panel uses so the two look alike.
const COLUMN_WEIGHTS: [f32; 6] = [2.0, 1.0, 1.0, 1.8, 1.6, 1.4];

/// What the panel shows beyond the solution itself.
pub struct Options<'a> {
    pub throws: bool,
    /// A line at the bottom, for saying what the overlay itself is doing.
    pub note: Option<&'a str>,
    /// Whether a button would do anything if pressed.
    pub permits: &'a dyn Fn(&Request) -> bool,
}

pub fn layout(solution: &Solution, config: &Config, text: &mut Text, options: &Options) -> Layout {
    let measured = build(solution, config, text, options, None);
    let edge = measured.width as f32 - config.window.padding as f32 * 2.0;
    build(solution, config, text, options, Some(edge))
}

/// `edge` is where the right-hand side of the panel is, once known.
fn build(
    solution: &Solution,
    config: &Config,
    text: &mut Text,
    options: &Options,
    edge: Option<f32>,
) -> Layout {
    let palette = &config.palette;
    let pad = config.window.padding as f32;
    let line = text.line_height();
    let mut builder = Builder {
        cells: Vec::new(),
        bands: Vec::new(),
        buttons: Vec::new(),
        y: pad,
        width: 0.0f32,
        pad,
        line,
        edge,
    };

    controls(text, &mut builder, options);

    if config.window.blind {
        if let Some(blind) = solution.blind.as_ref() {
            blind_panel(blind, config, text, &mut builder);
        }
    }

    match solution.placeholder() {
        Some(message) => {
            let message = message.to_owned();
            builder.row(&[cell(message, palette.base03)], text, None);
        }
        None if !solution.predictions.is_empty() => {
            predictions(solution, config, text, &mut builder)
        }
        None => {}
    }

    if options.throws && !solution.throws.is_empty() {
        throws(solution, config, text, &mut builder, options);
    }

    if config.window.messages {
        messages(solution, config, text, &mut builder);
    }

    if let Some(note) = options.note {
        builder.gap();
        builder.row(&[cell(note, palette.base0a)], text, None);
    }

    Layout {
        width: (builder.width + pad * 2.0).ceil().max(1.0) as u32,
        height: (builder.y + pad).ceil().max(1.0) as u32,
        bands: builder.bands,
        cells: builder.cells,
        buttons: builder.buttons,
    }
}

struct Builder {
    cells: Vec<Cell>,
    bands: Vec<Band>,
    buttons: Vec<Button>,
    y: f32,
    width: f32,
    pad: f32,
    line: f32,
    edge: Option<f32>,
}

impl Builder {
    /// Lays one line out left to right, and pushes `trailing` to the panel's
    /// right-hand edge, where it takes up width nobody was using.
    ///
    /// `reserve` is how much to keep clear at that edge. Giving it spreads the
    /// columns over the rest instead of leaving the slack in one lump, and
    /// passing the same value for a header as for the rows beneath keeps them
    /// lined up even though only the rows carry buttons.
    fn columns(
        &mut self,
        items: &[Item],
        widths: &[f32],
        text: &Text,
        band: Option<Color>,
        trailing: &[Item],
        reserve: Option<f32>,
    ) {
        if let Some(color) = band {
            self.bands.push(Band {
                y: self.y,
                height: self.line,
                color,
            });
        }
        let gap = self.spread(widths, text, reserve);
        let mut x = self.pad;
        for (item, width) in items.iter().zip(widths) {
            self.place(item, x, *width, text);
            x += width + gap;
        }
        let mut right = x - gap;

        if !trailing.is_empty() {
            let widths: Vec<f32> = trailing.iter().map(|item| item.width(text)).collect();
            let gaps = text.advance() * trailing.len().saturating_sub(1) as f32;
            let span: f32 = widths.iter().sum::<f32>() + gaps;
            // On the measuring pass there is no edge yet, so the trailing
            // items sit where they fall and count towards the width.
            let mut x = match self.edge {
                Some(edge) => (self.pad + edge - span).max(right + text.advance()),
                None => right + text.advance(),
            };
            for (item, width) in trailing.iter().zip(&widths) {
                self.place(item, x, *width, text);
                x += width + text.advance();
            }
            right = right.max(x - text.advance());
        }

        self.width = self.width.max(right - self.pad);
        self.y += self.line;
    }

    /// How far apart to set the columns so the row fills the panel.
    fn spread(&self, widths: &[f32], text: &Text, reserve: Option<f32>) -> f32 {
        let (Some(edge), Some(reserve)) = (self.edge, reserve) else {
            return text.advance();
        };
        let gaps = widths.len().saturating_sub(1);
        if gaps == 0 {
            return text.advance();
        }
        let natural = widths.iter().sum::<f32>() + text.advance() * gaps as f32;
        let slack = (edge - reserve - natural).max(0.0);
        text.advance() + slack / gaps as f32
    }

    fn place(&mut self, item: &Item, x: f32, width: f32, text: &Text) {
        match item {
            Item::Text(content, color) => {
                // Right-aligned: these are numbers, and the decimal points
                // should line up down the column.
                let offset = (width - text.width(content)).max(0.0);
                self.cells.push(Cell {
                    text: content.clone(),
                    color: *color,
                    x: x + offset,
                    y: self.y,
                });
            }
            Item::Press(label, request, enabled) => self.buttons.push(Button {
                label: label.clone(),
                request: request.clone(),
                enabled: *enabled,
                x,
                y: self.y,
                width,
                height: self.line,
            }),
        }
    }

    /// A line that is one run of items, each only as wide as it needs.
    fn row(&mut self, items: &[Item], text: &Text, band: Option<Color>) {
        let widths: Vec<f32> = items.iter().map(|item| item.width(text)).collect();
        self.columns(items, &widths, text, band, &[], None);
    }

    fn gap(&mut self) {
        self.y += self.line * 0.35;
    }
}

impl Item {
    fn width(&self, text: &Text) -> f32 {
        match self {
            Item::Text(content, _) => text.width(content),
            Item::Press(label, _, _) => text.width(label) + text.advance(),
        }
    }
}

/// Column widths that fit every row, respecting the panel's proportions.
fn widths(rows: &[Vec<Item>], text: &Text, weights: &[f32]) -> Vec<f32> {
    let count = rows.iter().map(Vec::len).max().unwrap_or(0);
    (0..count)
        .map(|column| {
            let content = rows
                .iter()
                .filter_map(|row| row.get(column))
                .map(|item| item.width(text))
                .fold(0.0f32, f32::max);
            let floor = weights.get(column).copied().unwrap_or(1.0) * text.advance() * 2.0;
            content.max(floor)
        })
        .collect()
}

/// The three buttons on a throw's row.
fn controls_for(throw: i32, options: &Options) -> [Item; 3] {
    [
        press("-", Request::Adjust(Index(throw), -1), options),
        press("+", Request::Adjust(Index(throw), 1), options),
        press("x", Request::Drop(Index(throw)), options),
    ]
}

/// How wide a run of items is, gaps included.
fn span(items: &[Item], text: &Text) -> f32 {
    let gaps = text.advance() * items.len().saturating_sub(1) as f32;
    items.iter().map(|item| item.width(text)).sum::<f32>() + gaps
}

fn press(label: &str, request: Request, options: &Options) -> Item {
    let enabled = (options.permits)(&request);
    Item::Press(label.to_owned(), request, enabled)
}

/// The top line: reset, undo and redo. Undo and redo are the box's, not the
/// calculator's.
fn controls(text: &mut Text, builder: &mut Builder, options: &Options) {
    let buttons = [
        press("reset", Request::Reset, options),
        press("undo", Request::Undo, options),
        press("redo", Request::Redo, options),
    ];
    let widths: Vec<f32> = buttons.iter().map(|item| item.width(text)).collect();
    // Quit sits at the far edge, in the space the table leaves, so that it is
    // nowhere near the three that get pressed in a hurry.
    let quit = [press("quit", Request::Quit, options)];
    builder.columns(&buttons, &widths, text, None, &quit, None);
}

fn blind_panel(blind: &Blind, config: &Config, text: &mut Text, builder: &mut Builder) {
    let palette = &config.palette;
    let mut rows = vec![
        vec![
            cell("Blind", palette.base03),
            cell(
                format!("{:.0}, {:.0}", blind.nether.0, blind.nether.1),
                palette.base0c,
            ),
        ],
        vec![
            cell("Quality", palette.base03),
            cell(
                blind.quality.label(),
                certainty_color(blind.quality.goodness(), palette),
            ),
        ],
        vec![
            cell("Highroll", palette.base03),
            cell(
                format!(
                    "{:.1}% under {:.0}",
                    blind.highroll_probability * 100.0,
                    blind.highroll_threshold
                ),
                palette.base0d,
            ),
        ],
    ];
    if let Some(improve) = blind.improve {
        rows.push(vec![
            cell("Improve", palette.base03),
            cell(
                format!("{:.0} blocks at {:.1}", improve.distance, improve.direction),
                palette.base0e,
            ),
        ]);
    }

    let widths = widths(&rows, text, &[2.0, 3.0]);
    for (index, row) in rows.iter().enumerate() {
        let band = (index == 0).then_some(palette.base01);
        builder.columns(row, &widths, text, band, &[], None);
    }
    builder.gap();
}

fn predictions(solution: &Solution, config: &Config, text: &mut Text, builder: &mut Builder) {
    let palette = &config.palette;
    let mut rows = vec![vec![
        cell(location_header(config), palette.base03),
        cell("%", palette.base03),
        cell("Dist.", palette.base03),
        cell("Nether", palette.base03),
        cell("Angle", palette.base03),
        cell("Turn", palette.base03),
    ]];
    for prediction in solution.predictions.iter().take(config.window.predictions) {
        let player = &solution.player;
        rows.push(vec![
            cell(
                format_pair(location(prediction, config)),
                coordinate_color(location(prediction, config), palette, config),
            ),
            cell(
                format!("{:.1}", prediction.certainty * 100.0),
                certainty_color(prediction.certainty, palette),
            ),
            cell(format!("{:.0}", prediction.distance(player)), palette.base0d),
            cell(format_pair(prediction.nether()), palette.base0c),
            cell(
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
    builder.columns(header, &widths, text, Some(palette.base01), &[], Some(0.0));
    for (index, row) in body.iter().enumerate() {
        let band = (index % 2 == 1).then_some(palette.base02);
        builder.columns(row, &widths, text, band, &[], Some(0.0));
    }
}

fn throws(
    solution: &Solution,
    config: &Config,
    text: &mut Text,
    builder: &mut Builder,
    options: &Options,
) {
    let palette = &config.palette;
    builder.gap();
    let correction_header = match config.window.angle_correction {
        AngleCorrection::Increments => "Steps",
        AngleCorrection::Degrees => "Corr.",
    };
    let mut rows = vec![vec![
        cell("x", palette.base03),
        cell("z", palette.base03),
        cell("Angle", palette.base03),
        cell(correction_header, palette.base03),
        cell("Error", palette.base03),
    ]];
    for throw in &solution.throws {
        rows.push(vec![
            cell(format!("{:.1}", throw.x), palette.base05),
            cell(format!("{:.1}", throw.z), palette.base05),
            cell(format!("{:.2}", throw.angle), palette.base0e),
            correction(throw, config, palette),
            cell(
                format!("{:.3}", throw.error),
                error_color(throw.error, palette),
            ),
        ]);
    }

    let widths = widths(&rows, text, &[1.5, 1.5, 1.5, 1.2, 1.5]);
    let Some((header, body)) = rows.split_first() else {
        return;
    };
    // The header carries no buttons but must keep their width clear, or its
    // columns would spread further than the rows beneath it.
    let reserve = Some(span(&controls_for(1, options), text));
    builder.columns(header, &widths, text, Some(palette.base01), &[], reserve);
    for (index, row) in body.iter().enumerate() {
        // The panel counts throws from one, and so does everything that talks
        // about them: the command line, the socket, and these buttons.
        let number = index as i32 + 1;
        let buttons = controls_for(number, options);
        builder.columns(row, &widths, text, None, &buttons, reserve);
    }
}

fn messages(solution: &Solution, config: &Config, text: &mut Text, builder: &mut Builder) {
    let palette = &config.palette;
    if solution.messages.is_empty() {
        return;
    }
    builder.gap();
    for message in &solution.messages {
        let color = match message.severity {
            Severity::Info => palette.base0d,
            Severity::Warning => palette.base09,
            Severity::Error => palette.base08,
        };
        let marker = match message.severity {
            Severity::Info => "i",
            _ => "!",
        };
        let width = config.window.message_width.max(8);
        for (index, line) in textwrap::wrap(&message.text, width).iter().enumerate()
        {
            let marker = if index == 0 { marker } else { " " };
            builder.row(
                &[cell(marker, color), cell(line.as_ref(), palette.base05)],
                text,
                None,
            );
        }
    }
}

fn location(prediction: &Prediction, config: &Config) -> (i32, i32) {
    match config.window.coordinates {
        Coordinates::Chunk => prediction.chunk,
        Coordinates::Block => prediction.block(),
    }
}

fn location_header(config: &Config) -> &'static str {
    match config.window.coordinates {
        Coordinates::Chunk => "Chunk",
        Coordinates::Block => "Location",
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

/// Red through to green as the calculator grows more sure.
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
fn turn(prediction: &Prediction, player: &crate::model::Player, palette: &Palette) -> Item {
    match prediction.travel_angle_delta(player) {
        None => cell(String::new(), palette.base03),
        Some(delta) => {
            let t = (delta.abs() as f32 / 30.0).clamp(0.0, 1.0);
            cell(format!("{delta:+.1}"), palette.base0b.mix(palette.base08, t))
        }
    }
}

/// How far a throw's angle was nudged, and which way. Signed and coloured,
/// because the sign is the whole point -- it says which way to correct next
/// time. Nothing is drawn when there was no correction.
fn correction(throw: &ThrowReport, config: &Config, palette: &Palette) -> Item {
    let (sign, content) = match config.window.angle_correction {
        AngleCorrection::Increments => (
            throw.correction_steps.signum(),
            format!("{:+}", throw.correction_steps),
        ),
        AngleCorrection::Degrees => (
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
        0 => cell(String::new(), palette.base03),
        1 => cell(content, palette.base0b),
        _ => cell(content, palette.base08),
    }
}

/// A throw's angle error, green while it is small.
fn error_color(error: f64, palette: &Palette) -> Color {
    let t = (error.abs() as f32 / 0.05).clamp(0.0, 1.0);
    palette.base0b.mix(palette.base08, t)
}

/// Draws a laid-out panel into `pixels`, which is a `wl_shm` buffer.
///
/// `wl_shm`'s `Argb8888` is B, G, R, A in memory on a little-endian machine,
/// which is tiny-skia's byte order with red and blue the other way round. So
/// the panel is drawn as tiny-skia likes it and the two channels are swapped
/// at the end, once, over the whole buffer.
pub fn draw(
    layout: &Layout,
    pixels: &mut [u8],
    size: (u32, u32),
    text: &mut Text,
    config: &Config,
    hovered: Option<usize>,
) {
    let (width, height) = size;
    let Some(mut pixmap) = PixmapMut::from_bytes(pixels, width, height) else {
        return;
    };
    let opacity = config.window.opacity;
    let palette = &config.palette;

    pixmap.fill(palette.base00.with_alpha(opacity).into());
    for band in &layout.bands {
        fill(&mut pixmap, 0.0, band.y, width as f32, band.height, band.color.with_alpha(opacity));
    }
    for (index, button) in layout.buttons.iter().enumerate() {
        let (face, ink) = match (button.enabled, hovered == Some(index)) {
            (false, _) => (palette.base01, palette.base03),
            (true, true) => (palette.base02.mix(palette.base03, 0.6), palette.base05),
            (true, false) => (palette.base02, palette.base05),
        };
        fill(
            &mut pixmap,
            button.x,
            button.y,
            button.width,
            button.height,
            face.with_alpha(opacity),
        );
        // Centred, because a button is a shape with a word in it rather than a
        // column of digits to line up.
        let inset = ((button.width - text.width(&button.label)) / 2.0).max(0.0);
        glyphs(&mut pixmap, text, &button.label, button.x + inset, button.y, ink);
    }
    for cell in &layout.cells {
        glyphs(&mut pixmap, text, &cell.text, cell.x, cell.y, cell.color);
    }

    for pixel in pixels.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }
}

fn fill(pixmap: &mut PixmapMut, x: f32, y: f32, width: f32, height: f32, color: Color) {
    let Some(rect) = Rect::from_xywh(x, y, width, height) else {
        return;
    };
    let paint = Paint {
        shader: Shader::SolidColor(color.into()),
        ..Paint::default()
    };
    pixmap.fill_rect(rect, &paint, Transform::identity(), None);
}

/// Each glyph is rasterised to a coverage bitmap, turned into a little pixmap
/// of the ink colour, and composited.
fn glyphs(pixmap: &mut PixmapMut, text: &mut Text, content: &str, x: f32, y: f32, color: Color) {
    let advance = text.advance();
    let baseline = y + text.ascent();
    let paint = PixmapPaint::default();
    for (index, character) in content.chars().enumerate() {
        let (metrics, coverage) = text.glyph(character);
        let (width, height) = (metrics.width as u32, metrics.height as u32);
        let Some(mut glyph) = Pixmap::new(width, height) else {
            continue;
        };
        for (pixel, coverage) in glyph.pixels_mut().iter_mut().zip(coverage) {
            *pixel = color.with_alpha(*coverage as f32 / 255.0).into();
        }
        let left = (x + advance * index as f32).round() as i32 + metrics.xmin;
        let top = baseline.round() as i32 - metrics.height as i32 - metrics.ymin;
        pixmap.draw_pixmap(left, top, glyph.as_ref(), &paint, Transform::identity(), None);
    }
}
