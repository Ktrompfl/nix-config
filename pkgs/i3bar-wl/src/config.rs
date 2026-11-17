use serde::Deserialize;
use std::{collections::HashMap, io::ErrorKind, path::PathBuf};

#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub command: Vec<String>,
    pub font: String,
    pub height: u32,
    pub position: Position,
    pub env: HashMap<String, String>,
    pub colors: Colors,
}

#[derive(Clone, Copy, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Position {
    Top,
    Bottom,
}

#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Colors {
    pub background: Color,
    pub text: Color,
    pub separator: Color,
    pub workspace: Color,
    pub workspace_text: Color,
    pub active: Color,
    pub active_text: Color,
    pub urgent: Color,
    pub urgent_text: Color,
    pub menu: Color,
    pub menu_border: Color,
    pub menu_disabled: Color,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(try_from = "String")]
pub struct Color(pub u32);

impl Color {
    pub fn parse(hex: &str) -> Option<Self> {
        let hex = hex.strip_prefix('#').filter(|h| h.bytes().all(|b| b.is_ascii_hexdigit()))?;
        let value = u32::from_str_radix(hex, 16).ok()?;
        match hex.len() {
            6 => Some(Self(0xff00_0000 | value)),
            8 => Some(Self(value.rotate_right(8))),
            _ => None,
        }
    }
}

impl TryFrom<String> for Color {
    type Error = String;

    fn try_from(s: String) -> Result<Self, String> {
        Self::parse(&s).ok_or_else(|| format!("invalid color {s:?}, expected \"#rrggbb\" or \"#rrggbbaa\""))
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            command: vec!["i3status-rs".into()],
            font: "sans-serif 10".into(),
            height: 18,
            position: Position::Bottom,
            env: HashMap::new(),
            colors: Colors::default(),
        }
    }
}

impl Default for Colors {
    fn default() -> Self {
        Self {
            background: Color(0xff1d2021),
            text: Color(0xffa89984),
            separator: Color(0xff504945),
            workspace: Color(0xff32302f),
            workspace_text: Color(0xffa89984),
            active: Color(0xff83a598),
            active_text: Color(0xff1d2021),
            urgent: Color(0xfffb4934),
            urgent_text: Color(0xff1d2021),
            menu: Color(0xff282828),
            menu_border: Color(0xff504945),
            menu_disabled: Color(0xff665c54),
        }
    }
}

impl Config {
    pub fn load(explicit: Option<PathBuf>) -> Result<Self, String> {
        let path = match explicit.clone() {
            Some(path) => path,
            None => {
                let base = std::env::var_os("XDG_CONFIG_HOME").filter(|d| !d.is_empty()).map(PathBuf::from);
                let base = base.or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")));
                base.ok_or("neither XDG_CONFIG_HOME nor HOME is set")?.join("i3bar-wl/config.toml")
            }
        };
        let text = match std::fs::read_to_string(&path) {
            Err(e) if e.kind() == ErrorKind::NotFound && explicit.is_none() => return Ok(Self::default()),
            text => text.map_err(|e| format!("cannot read {}: {e}", path.display()))?,
        };
        let config: Self = toml::from_str(&text).map_err(|e| format!("invalid config {}: {e}", path.display()))?;
        match config.height {
            0 => Err(format!("invalid config {}: height must be positive", path.display())),
            _ => Ok(config),
        }
    }
}
