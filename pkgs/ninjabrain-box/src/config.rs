//! The TOML the overlay is configured with, and its defaults.

use std::path::PathBuf;

use serde::Deserialize;

/// An edge of an output the window can be pinned to. Naming one edge leaves
/// the window centred along it, so `["left"]` is the left-and-centre of the
/// examples and `["top", "right"]` is the top-right corner.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Edge {
    Top,
    Bottom,
    Left,
    Right,
}

/// Which coordinates name a stronghold.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Coordinates {
    Chunk,
    Block,
}

/// How a throw's angle correction is counted.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum AngleCorrection {
    /// Adjustment steps, which is what there is to press.
    Increments,
    /// The angle it comes to.
    Degrees,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct Config {
    pub bot: Bot,
    pub window: Window,
    pub behavior: Behavior,
    pub palette: Palette,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct Bot {
    /// Where the bot's HTTP API listens. The port is the bot's own.
    pub api: String,
    pub settings: Settings,
}

/// The bot's own settings, as far as they still matter.
///
/// Everything here changes what the bot calculates. What it drew, in what
/// colours, at what size and under which hotkeys is gone: the overlay draws
/// the panel and drives the actions itself.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct Settings {
    pub mc_version: McVersion,
    pub angle_adjustment: AngleAdjustment,
    pub boat_type: BoatType,
    pub all_advancements: bool,
    pub all_advancements_toggle: AllAdvancementsToggle,
    pub all_advancements_1_20_plus: bool,
    pub use_precise_angle: bool,
    pub use_alt_std: bool,
    pub use_advanced_statistics: bool,
    /// In-game mouse sensitivity.
    pub sensitivity: f64,
    pub sensitivity_manual: f64,
    /// Assumed standard deviation of a measurement.
    pub sigma: f64,
    pub sigma_alt: f64,
    pub sigma_manual: f64,
    pub sigma_boat: f64,
    pub boat_error: f64,
    /// Height the game renders at, for the subpixel adjustment.
    pub resolution_height: f64,
    pub custom_adjustment: f64,
    pub crosshair_correction: f64,
    /// Reset after fifteen idle minutes.
    pub auto_reset: bool,
    pub auto_reset_on_instance_change: bool,
    pub save_state: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub enum McVersion {
    #[serde(rename = "pre-1.19")]
    Pre119,
    #[serde(rename = "1.19+")]
    Post119,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum AngleAdjustment {
    Subpixel,
    Tall,
    Custom,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum BoatType {
    Gray,
    Blue,
    Green,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum AllAdvancementsToggle {
    Automatic,
    Hotkey,
}

impl Settings {
    /// The preference entries these settings amount to. The bot stores a
    /// choice as its position in the list the GUI offers, so the enums come
    /// out as indices.
    pub fn entries(&self) -> Vec<(String, String)> {
        let flag = |value: bool| value.to_string();
        vec![
            ("mc_version".into(), (self.mc_version as u8).to_string()),
            ("angle_adjustment_type".into(), (self.angle_adjustment as u8).to_string()),
            ("default_boat_type".into(), (self.boat_type as u8).to_string()),
            ("all_advancements".into(), flag(self.all_advancements)),
            ("aa_toggle_type".into(), (self.all_advancements_toggle as u8).to_string()),
            ("one_dot_twenty_plus_aa".into(), flag(self.all_advancements_1_20_plus)),
            ("use_precise_angle".into(), flag(self.use_precise_angle)),
            ("use_alt_std".into(), flag(self.use_alt_std)),
            ("use_adv_statistics".into(), flag(self.use_advanced_statistics)),
            ("sensitivity".into(), self.sensitivity.to_string()),
            ("sensitivity_manual".into(), self.sensitivity_manual.to_string()),
            ("sigma".into(), self.sigma.to_string()),
            ("sigma_alt".into(), self.sigma_alt.to_string()),
            ("sigma_manual".into(), self.sigma_manual.to_string()),
            ("sigma_boat".into(), self.sigma_boat.to_string()),
            ("boat_error".into(), self.boat_error.to_string()),
            ("resolution_height".into(), self.resolution_height.to_string()),
            ("custom_adjustment".into(), self.custom_adjustment.to_string()),
            ("crosshair_correction".into(), self.crosshair_correction.to_string()),
            ("auto_reset".into(), flag(self.auto_reset)),
            ("auto_reset_on_instance_change".into(), flag(self.auto_reset_on_instance_change)),
            ("save_state".into(), flag(self.save_state)),
        ]
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct Window {
    /// The output to pin to, by connector name, or none for whichever the
    /// compositor picks.
    pub output: Option<String>,
    pub anchor: Vec<Edge>,
    /// Gap to the anchored edges, in the same order as `anchor` names them:
    /// top, right, bottom, left.
    pub margin: [i32; 4],
    pub coordinates: Coordinates,
    pub angle_correction: AngleCorrection,
    /// How many predictions to show. The API returns the bot's top few.
    pub predictions: usize,
    pub font: Option<PathBuf>,
    pub font_size: f32,
    pub padding: i32,
    /// Draw negative coordinates in the palette's red, as the bot does.
    pub color_negative_coordinates: bool,
    /// Drawn behind everything, as an alpha over the palette's background.
    pub opacity: f32,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct Behavior {
    /// Whether the window is on screen when the overlay starts.
    pub start_hidden: bool,
    /// Whether the throw table is part of it to begin with. Off means
    /// something has to ask for it; the predictions are always there.
    pub start_with_throws: bool,
    /// Only ever show while one of these is the focused window. Matched
    /// against the app id first and the title second, as a substring. Empty
    /// means no restriction.
    pub only_when_focused: Vec<String>,
}

/// A base16 palette. The names are the scheme's, the uses are ours.
#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Palette {
    /// Background.
    pub base00: Color,
    /// Header and divider background.
    pub base01: Color,
    /// Alternating row background.
    pub base02: Color,
    /// Muted text: headers, labels.
    pub base03: Color,
    pub base04: Color,
    /// Default text.
    pub base05: Color,
    pub base06: Color,
    pub base07: Color,
    /// Low certainty, negative coordinates, large angle corrections.
    pub base08: Color,
    /// Middling certainty.
    pub base09: Color,
    /// Nearly certain.
    #[serde(alias = "base0A")]
    pub base0a: Color,
    /// Certain, and small angle corrections.
    #[serde(alias = "base0B")]
    pub base0b: Color,
    /// Nether coordinates.
    #[serde(alias = "base0C")]
    pub base0c: Color,
    /// Distances.
    #[serde(alias = "base0D")]
    pub base0d: Color,
    /// Travel angles.
    #[serde(alias = "base0E")]
    pub base0e: Color,
    #[serde(alias = "base0F")]
    pub base0f: Color,
}

/// An `#rrggbb` or `#rrggbbaa` colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Color(pub u8, pub u8, pub u8, pub u8);

impl Color {
    const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Color(r, g, b, 0xff)
    }

    /// Straight-line blend, for the certainty gradient.
    pub fn mix(self, other: Color, t: f32) -> Color {
        let t = t.clamp(0.0, 1.0);
        let lerp = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
        Color(
            lerp(self.0, other.0),
            lerp(self.1, other.1),
            lerp(self.2, other.2),
            lerp(self.3, other.3),
        )
    }

    pub fn with_alpha(self, alpha: f32) -> Color {
        Color(self.0, self.1, self.2, (self.3 as f32 * alpha).round() as u8)
    }
}

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        parse_color(&text).ok_or_else(|| {
            serde::de::Error::custom(format!("expected #rrggbb or #rrggbbaa, got {text:?}"))
        })
    }
}

fn parse_color(text: &str) -> Option<Color> {
    let digits = text.strip_prefix('#').unwrap_or(text);
    if digits.len() != 6 && digits.len() != 8 {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(digits.get(i..i + 2)?, 16).ok();
    Some(Color(
        byte(0)?,
        byte(2)?,
        byte(4)?,
        if digits.len() == 8 { byte(6)? } else { 0xff },
    ))
}

impl Default for Bot {
    fn default() -> Self {
        Bot {
            api: "127.0.0.1:52533".into(),
            settings: Settings::default(),
        }
    }
}

/// The bot's own defaults, so that an unconfigured overlay calculates exactly
/// as a stock bot would.
impl Default for Settings {
    fn default() -> Self {
        Settings {
            mc_version: McVersion::Pre119,
            angle_adjustment: AngleAdjustment::Subpixel,
            boat_type: BoatType::Gray,
            all_advancements: false,
            all_advancements_toggle: AllAdvancementsToggle::Automatic,
            all_advancements_1_20_plus: false,
            use_precise_angle: false,
            use_alt_std: false,
            use_advanced_statistics: true,
            sensitivity: 0.012727597,
            sensitivity_manual: 0.4341732,
            sigma: 0.1,
            sigma_alt: 0.1,
            sigma_manual: 0.03,
            sigma_boat: 0.001,
            boat_error: 0.03,
            resolution_height: 16384.0,
            custom_adjustment: 0.01,
            crosshair_correction: 0.0,
            auto_reset: false,
            auto_reset_on_instance_change: false,
            save_state: true,
        }
    }
}

impl Default for Window {
    fn default() -> Self {
        Window {
            output: None,
            anchor: vec![Edge::Top, Edge::Right],
            margin: [8, 8, 8, 8],
            coordinates: Coordinates::Chunk,
            angle_correction: AngleCorrection::Increments,
            predictions: 4,
            font: None,
            font_size: 15.0,
            padding: 6,
            color_negative_coordinates: true,
            opacity: 0.85,
        }
    }
}

/// Default Sky, the base16 scheme, so that an unconfigured overlay is legible.
impl Default for Palette {
    fn default() -> Self {
        Palette {
            base00: Color::rgb(0x16, 0x16, 0x16),
            base01: Color::rgb(0x1f, 0x1f, 0x1f),
            base02: Color::rgb(0x2a, 0x2a, 0x2a),
            base03: Color::rgb(0x6c, 0x6c, 0x6c),
            base04: Color::rgb(0x8f, 0x8f, 0x8f),
            base05: Color::rgb(0xd8, 0xd8, 0xd8),
            base06: Color::rgb(0xe8, 0xe8, 0xe8),
            base07: Color::rgb(0xf8, 0xf8, 0xf8),
            base08: Color::rgb(0xe0, 0x64, 0x64),
            base09: Color::rgb(0xe0, 0x9a, 0x5a),
            base0a: Color::rgb(0xe0, 0xd0, 0x64),
            base0b: Color::rgb(0x7c, 0xc9, 0x6e),
            base0c: Color::rgb(0x6e, 0xc9, 0xc0),
            base0d: Color::rgb(0x6e, 0x9c, 0xc9),
            base0e: Color::rgb(0xb4, 0x8e, 0xc9),
            base0f: Color::rgb(0xa1, 0x6a, 0x4b),
        }
    }
}


impl Config {
    /// Reads the configuration, falling back to the defaults if there is none.
    pub fn load() -> Result<Config, String> {
        let Some(path) = default_path().filter(|path| path.exists()) else {
            return Ok(Config::default());
        };
        let text = std::fs::read_to_string(&path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        toml::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))
    }
}

fn default_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    Some(base.join("ninjabrain-box/config.toml"))
}
