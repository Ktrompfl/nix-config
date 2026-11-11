//! find names and models of outputs with `jay randr`

use jay_config::{
    get_workspace,
    video::{
        Connector, Mode, Transform, VrrMode, connectors, on_connector_connected,
        on_connector_disconnected,
    },
};

use crate::mirror;

pub enum Match {
    Connector(&'static str),
    Model(&'static str),
}

pub const LAPTOP_PANEL: Match = Match::Connector("eDP-1");

impl Match {
    fn matches(&self, connector: Connector) -> bool {
        match *self {
            Match::Connector(name) => connector.name() == name,
            Match::Model(model) => connector.model() == model,
        }
    }

    pub fn find(&self) -> Option<Connector> {
        connectors()
            .into_iter()
            .find(|&c| c.connected() && self.matches(c))
    }

    /// The connected output, or one that does not exist.
    pub fn connector(&self) -> Connector {
        self.find().unwrap_or(Connector(0))
    }
}

struct Output {
    position: (i32, i32),
    /// width, height, refresh rate in Hz; skipped unless the output offers it
    mode: (i32, i32, f64),
    scale: Option<f64>,
    transform: Option<Transform>,
    vrr: Option<VrrMode>,
    /// the workspaces that start out here, unless an earlier output has them
    workspaces: &'static [&'static str],
}

const PANEL: Output = Output {
    position: (0, 0),
    mode: (1920, 1080, 60.049),
    scale: None,
    transform: None,
    vrr: None,
    workspaces: &["1", "2", "3", "4", "5"],
};

/// Any other output while the laptop panel is connected: below the panel,
/// mirroring it (see `mirror`).
const MIRRORED: Output = Output {
    position: (0, 1080),
    mode: (1920, 1080, 60.0),
    scale: Some(1.0),
    transform: Some(Transform::None),
    vrr: None,
    workspaces: &["0"],
};

/// Outputs of machines without a laptop panel.
const OUTPUTS: &[(Match, Output)] = &[
    (
        Match::Model("VG270U P"),
        Output {
            position: (0, 240),
            mode: (2560, 1440, 143.995),
            scale: Some(1.0),
            transform: Some(Transform::None),
            // if this does not work try VARIANT_2
            vrr: Some(VrrMode::VARIANT_3),
            workspaces: &["1", "2", "3", "4", "5"],
        },
    ),
    (
        Match::Model("BenQ GL2480"),
        Output {
            position: (2560, 0),
            mode: (1920, 1080, 60.0),
            scale: Some(1.0),
            transform: Some(Transform::Rotate90),
            vrr: None,
            workspaces: &["6", "7", "8", "9", "0"],
        },
    ),
];

/// The configuration of `connector` and its rank; lower ranks win workspaces.
fn config(connector: Connector) -> Option<(usize, &'static Output)> {
    if LAPTOP_PANEL.matches(connector) {
        return Some((0, &PANEL));
    }
    if LAPTOP_PANEL.find().is_some() {
        return Some((1, &MIRRORED));
    }
    OUTPUTS
        .iter()
        .position(|(m, _)| m.matches(connector))
        .map(|i| (i + 2, &OUTPUTS[i].1))
}

impl Output {
    fn apply(&self, connector: Connector) {
        let (x, y) = self.position;
        connector.set_position(x, y);
        let (width, height, hz) = self.mode;
        // the same output offers different modes over HDMI and DisplayPort,
        // and the rate may be off by rounding
        let millihz = (hz * 1000.0).round() as i64;
        let offset = |m: &Mode| (i64::from(m.refresh_rate()) - millihz).abs();
        let closest = connector
            .modes()
            .into_iter()
            .filter(|m| m.width() == width && m.height() == height)
            .min_by_key(offset)
            .filter(|m| offset(m) < 500);
        match closest {
            Some(m) => connector.set_mode(width, height, Some(m.refresh_rate())),
            None => log::warn!(
                "{} does not offer {width}x{height}@{hz}, keeping its mode",
                connector.name()
            ),
        }
        if let Some(scale) = self.scale {
            connector.set_scale(scale);
        }
        if let Some(transform) = self.transform {
            connector.set_transform(transform);
        }
        if let Some(vrr) = self.vrr {
            connector.set_vrr_mode(vrr);
        }
    }
}

/// `disconnected` is still reported as connected while its event is handled.
fn assign_workspaces(disconnected: Option<Connector>) {
    let mut configured: Vec<_> = connectors()
        .into_iter()
        .filter(|&c| c.connected() && Some(c) != disconnected)
        .filter_map(|c| config(c).map(|(rank, output)| (rank, c, output)))
        .collect();
    // back to front, so that lower ranks win
    configured.sort_by_key(|&(rank, _, _)| std::cmp::Reverse(rank));
    for (_, connector, output) in configured {
        for workspace in output.workspaces {
            get_workspace(workspace).set_initial_connector(Some(connector));
        }
    }
}

pub fn setup() {
    on_connector_connected(|connector| {
        if let Some((_, output)) = config(connector) {
            output.apply(connector);
        }
        // outputs connected before the panel were configured as if it was absent
        if LAPTOP_PANEL.matches(connector) {
            for other in connectors() {
                if other.connected() && other != connector {
                    MIRRORED.apply(other);
                }
            }
        }
        assign_workspaces(None);
        mirror::update(None);
    });
    on_connector_disconnected(|connector| {
        assign_workspaces(Some(connector));
        mirror::update(Some(connector));
    });
}
