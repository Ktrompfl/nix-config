//! The TOML the overlay is configured with, and its defaults.

use anyhow::{bail, Context, Result};
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

/// When the overlay draws the buttons that drive the bot.
///
/// Anything other than `off` means the surface has to accept pointer input,
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

/// Declares a settings block once, instead of naming every setting in a
/// struct, a default and a converter. `index` is for the bot's multiple-choice
/// preferences, which it stores as a position in the list its GUI offers.
macro_rules! settings {
    ($($field:ident: $type:ty = $default:expr => $key:literal as $how:ident),* $(,)?) => {
        /// The calculator's own settings, as far as a person would change them.
        ///
        /// Everything the box fixes is absent: it measures with a green boat
        /// eye and nothing else, so the boat type, the precise-angle switch and
        /// the adjustment step are not choices, and the settings that only
        /// apply to other ways of measuring have no effect at all.
        #[derive(Clone, Debug, Deserialize, PartialEq)]
        #[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
        pub struct Settings {
            $(pub $field: $type),*
        }

        impl Default for Settings {
            fn default() -> Settings {
                Settings { $($field: $default),* }
            }
        }

        impl Settings {
            fn chosen(&self) -> Vec<(String, String)> {
                vec![$(($key.to_owned(), settings!(@as $how self.$field))),*]
            }
        }
    };
    (@as index $value:expr) => { ($value as u8).to_string() };
    (@as value $value:expr) => { $value.to_string() };
}

settings! {
    mc_version: McVersion = McVersion::Pre119 => "mc_version" as index,
    sensitivity: f64 = GOD_SENSE[0] => "sensitivity" as value,
    sigma_boat: f64 = 0.001 => "sigma_boat" as value,
    crosshair_correction: f64 = 0.0 => "crosshair_correction" as value,
    use_advanced_statistics: bool = true => "use_adv_statistics" as value,
    auto_reset: bool = false => "auto_reset" as value,
    save_state: bool = true => "save_state" as value,
}

/// The sensitivities a green boat eye can be measured at.
///
/// Entering a boat sets the player's yaw to the boat's, which is a multiple of
/// 360/256 degrees; turning from there moves it in whole multiples of the
/// smallest increment the mouse can produce. The bot recovers the exact angle
/// by snapping what F3+C reports onto that grid -- but with a green eye it
/// anchors the grid at zero rather than measuring the boat, which is only the
/// same grid when the increment divides 360/256 exactly. These are the
/// sensitivities where it does, dividing it into 120, 90 and 80 steps.
///
/// At any other sensitivity the snap silently lands on the wrong grid point
/// and every measurement is quietly wrong, so this is checked rather than
/// documented.
pub const GOD_SENSE: [f64; 3] = [0.02291165, 0.058765005, 0.07446537];

/// The smallest angle a mouse movement can turn through, which is also the
/// step the eye throw adjustment should move in.
pub fn minimum_increment(sensitivity: f64) -> f64 {
    let factor = sensitivity * 0.6 + 0.2;
    factor * factor * factor * 8.0 * 0.15
}

impl Settings {
    /// The preference entries these settings amount to, including the ones the
    /// box fixes rather than offers.
    pub fn entries(&self) -> Vec<(String, String)> {
        let mut entries = self.chosen();
        // A green boat eye, always: the angle grid is anchored at zero and no
        // boat angle is ever measured, which is what makes the boat hotkey,
        // the other two eye colours and the modulo-360 correction pointless.
        entries.push(("default_boat_type".into(), "2".into()));
        entries.push(("use_precise_angle".into(), "true".into()));
        // One press of the adjustment should move one mouse increment, which
        // depends on the sensitivity, so it is set rather than chosen.
        entries.push(("angle_adjustment_type".into(), "2".into()));
        entries.push((
            "custom_adjustment".into(),
            minimum_increment(self.sensitivity).to_string(),
        ));
        entries
    }

    /// Refuses a sensitivity that would make every measurement wrong.
    pub fn check(&self) -> Result<()> {
        if GOD_SENSE.contains(&self.sensitivity) {
            return Ok(());
        }
        let names: Vec<String> = GOD_SENSE.iter().map(f64::to_string).collect();
        bail!(
            "sensitivity must be one of {} -- a green boat eye only measures \
             correctly at a sensitivity whose smallest angle increment divides \
             the boat's, and {} is not one",
            names.join(", "),
            self.sensitivity
        )
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub enum McVersion {
    #[serde(rename = "pre-1.19")]
    Pre119,
    #[serde(rename = "1.19+")]
    Post119,
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
    /// Show what the bot has to say: mismeasurement warnings, portal linking,
    /// which way to walk before the next throw.
    pub messages: bool,
    /// How wide a message may be before it is wrapped, in characters.
    pub message_width: usize,
    /// Show the blind travel recommendation when the bot is in blind mode.
    pub blind: bool,
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
    /// Only show while one of these is the focused window. Matched against
    /// the app id first and the title second, as a substring. Empty means no
    /// restriction. `show`, `hide` and `toggle` override this until one of
    /// these windows is focused again.
    pub only_when_focused: Vec<String>,
    /// How much the backend may do to the calculator. `restricted` allows only
    /// what the calculator offers a user directly; `unbound` allows the box to
    /// replay measurements it has already seen, which is what makes editing a
    /// throw that is not the last one possible at all.
    pub mode: crate::backend::Mode,
    /// Take the window away entirely while the bot has nothing to say.
    pub auto_hide: bool,
}

/// Declares the palette once. The names are base16's; what each is used for
/// is in the README, next to where a person would be choosing them.
macro_rules! palette {
    ($($field:ident = $default:literal $(as $alias:literal)?),* $(,)?) => {
        /// A base16 palette.
        #[derive(Clone, Debug, Deserialize)]
        #[serde(default, deny_unknown_fields)]
        pub struct Palette {
            $(
                $(#[serde(alias = $alias)])?
                pub $field: Color,
            )*
        }

        impl Default for Palette {
            fn default() -> Palette {
                Palette { $($field: Color::rgb($default)),* }
            }
        }
    };
}

palette! {
    base00 = 0x161616,
    base01 = 0x1f1f1f,
    base02 = 0x2a2a2a,
    base03 = 0x6c6c6c,
    base04 = 0x8f8f8f,
    base05 = 0xd8d8d8,
    base06 = 0xe8e8e8,
    base07 = 0xf8f8f8,
    base08 = 0xe06464,
    base09 = 0xe09a5a,
    // Schemes are written with these six in upper case, so both spellings
    // are taken.
    base0a = 0xe0d064 as "base0A",
    base0b = 0x7cc96e as "base0B",
    base0c = 0x6ec9c0 as "base0C",
    base0d = 0x6e9cc9 as "base0D",
    base0e = 0xb48ec9 as "base0E",
    base0f = 0xa16a4b as "base0F",
}

/// An `#rrggbb` or `#rrggbbaa` colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Color(pub u8, pub u8, pub u8, pub u8);

impl Color {
    const fn rgb(hex: u32) -> Self {
        Color((hex >> 16) as u8, (hex >> 8) as u8, hex as u8, 0xff)
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

/// tiny-skia wants premultiplied colour; the palette is written straight.
impl From<Color> for tiny_skia::Color {
    fn from(Color(r, g, b, a): Color) -> tiny_skia::Color {
        tiny_skia::Color::from_rgba8(r, g, b, a)
    }
}

impl From<Color> for tiny_skia::PremultipliedColorU8 {
    fn from(color: Color) -> tiny_skia::PremultipliedColorU8 {
        tiny_skia::Color::from(color).premultiply().to_color_u8()
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
            messages: true,
            message_width: 44,
            blind: true,
        }
    }
}


impl Config {
    /// Reads the configuration, falling back to the defaults if there is none.
    pub fn load() -> Result<Config> {
        let Some(path) = default_path().filter(|path| path.exists()) else {
            return Ok(Config::default());
        };
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("cannot read {}", path.display()))?;
        let config: Config =
            toml::from_str(&text).with_context(|| format!("{}", path.display()))?;
        config.bot.settings.check()?;
        Ok(config)
    }
}

/// Where the configuration lives, if it exists.
fn default_path() -> Option<PathBuf> {
    directories().find_config_file("config.toml")
}

/// This overlay's corner of the XDG base directories.
pub fn directories() -> xdg::BaseDirectories {
    xdg::BaseDirectories::with_prefix("ninjabrain-box")
        .expect("the XDG base directories always resolve on Linux")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A green boat eye only measures correctly where the smallest mouse
    /// increment divides the boat's own step of 360/256 degrees.
    #[test]
    fn god_sense_divides_the_boats_step() {
        const BOAT_STEP: f64 = 360.0 / 256.0;
        for sensitivity in GOD_SENSE {
            let steps = BOAT_STEP / minimum_increment(sensitivity);
            assert!(
                (steps - steps.round()).abs() < 1e-4,
                "{sensitivity} gives {steps} steps per boat step"
            );
        }
        // The bot's own default does not, which is the whole reason for the
        // check.
        let steps = BOAT_STEP / minimum_increment(0.012727597);
        assert!((steps - steps.round()).abs() > 0.05, "{steps} is too close to whole");
    }

    /// base16 schemes spell the last six in upper case, and a palette that
    /// does is the normal case rather than the exception.
    #[test]
    fn a_palette_may_be_written_in_upper_case() {
        let config: Config = toml::from_str(
            r##"
            [palette]
            base09 = "#111111"
            base0A = "#222222"
            base0F = "#333333"
            "##,
        )
        .expect("parses");
        assert_eq!(config.palette.base09, Color(0x11, 0x11, 0x11, 0xff));
        assert_eq!(config.palette.base0a, Color(0x22, 0x22, 0x22, 0xff));
        assert_eq!(config.palette.base0f, Color(0x33, 0x33, 0x33, 0xff));
        // The lower-case spelling still works too.
        let config: Config = toml::from_str("[palette]\nbase0a = \"#444444\"\n").expect("parses");
        assert_eq!(config.palette.base0a, Color(0x44, 0x44, 0x44, 0xff));
    }

    #[test]
    fn any_other_sensitivity_is_refused() {
        let mut settings = Settings::default();
        assert!(settings.check().is_ok());
        settings.sensitivity = 0.5;
        assert!(settings.check().is_err());
    }

    /// One press of the adjustment should move exactly one mouse increment,
    /// so the box sets it rather than offering it.
    #[test]
    fn the_adjustment_step_follows_the_sensitivity() {
        let settings = Settings {
            sensitivity: GOD_SENSE[1],
            ..Settings::default()
        };
        let entries = settings.entries();
        let written = |key: &str| {
            entries
                .iter()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value.clone())
                .expect("written")
        };
        assert_eq!(written("angle_adjustment_type"), "2");
        assert_eq!(written("custom_adjustment"), minimum_increment(GOD_SENSE[1]).to_string());
        // And the eye is always green, with the precise angle it needs.
        assert_eq!(written("default_boat_type"), "2");
        assert_eq!(written("use_precise_angle"), "true");
    }
}
