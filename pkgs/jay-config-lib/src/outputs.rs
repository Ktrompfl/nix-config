//! find names and models of outputs with `jay randr`
//!
//! Mode, position, scale and transform are left to shikane (see
//! home/graphical/wayland/outputs.nix); this only sets what
//! wlr-output-management cannot express, and assigns workspaces.

use jay_config::{
    get_workspace,
    video::{
        Connector, Format, VrrMode, connectors, on_connector_connected, on_connector_disconnected,
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
    vrr: Option<VrrMode>,
    /// assume the primaries advertised in the EDID instead of sRGB
    native_gamut: Option<bool>,
    format: Option<Format>,
    /// the workspaces that start out here, unless an earlier output has them
    workspaces: &'static [&'static str],
}

const PANEL: Output = Output {
    vrr: None,
    native_gamut: None,
    format: None,
    workspaces: &["1", "2", "3", "4", "5"],
};

/// Any other output while the laptop panel is connected: mirroring it (see
/// `mirror`).
const MIRRORED: Output = Output {
    vrr: None,
    native_gamut: None,
    format: None,
    workspaces: &["0"],
};

/// Outputs of machines without a laptop panel.
const OUTPUTS: &[(Match, Output)] = &[
    (
        Match::Model("VG270U P"),
        Output {
            // if this does not work try VARIANT_2
            vrr: Some(VrrMode::VARIANT_3),
            native_gamut: Some(true),
            format: Some(Format::XRGB2101010),
            workspaces: &["1", "2", "3", "4", "5"],
        },
    ),
    (
        Match::Model("BenQ GL2480"),
        Output {
            vrr: None,
            native_gamut: None,
            format: None,
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
        if let Some(vrr) = self.vrr {
            connector.set_vrr_mode(vrr);
        }
        if let Some(native_gamut) = self.native_gamut {
            connector.set_use_native_gamut(native_gamut);
        }
        if let Some(format) = self.format {
            connector.set_format(format);
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
        assign_workspaces(None);
        mirror::update(None);
    });
    on_connector_disconnected(|connector| {
        assign_workspaces(Some(connector));
        mirror::update(Some(connector));
    });
}
