//! The displays and which of them a workspace starts out on.
//!
//! find names of outputs with 'jay randr'

use jay_config::{
    get_workspace,
    video::{
        Connector, Transform, VrrMode, connectors, on_connector_connected,
        on_connector_disconnected,
    },
};

/// How an output is recognized. The laptop panel is always on the same
/// connector; the external ones move between ports and are recognized by
/// their model.
enum Match {
    Connector(&'static str),
    Model(&'static str),
}

struct Output {
    /// Also how the other modules refer to this output, see [`connector`].
    name: &'static str,
    match_: Match,
    position: (i32, i32),
    /// width, height, and refresh rate in Hz
    mode: (i32, i32, f64),
    scale: Option<f64>,
    transform: Option<Transform>,
    vrr: Option<VrrMode>,
    /// The workspaces that start out here, see [`assign_initial_connectors`].
    workspaces: &'static [&'static str],
}

const OUTPUTS: &[Output] = &[
    Output {
        name: "laptop-integrated",
        match_: Match::Connector("eDP-1"),
        position: (0, 0),
        mode: (1920, 1080, 60.0),
        scale: None,
        transform: None,
        vrr: None,
        workspaces: &["1", "2", "3", "4", "5"],
    },
    Output {
        // TODO: automatically spawn wl-mirror
        name: "beamer",
        match_: Match::Model("EPSON PJ"),
        position: (0, 1080),
        mode: (1920, 1080, 60.0),
        scale: None,
        transform: None,
        vrr: None,
        workspaces: &["0"],
    },
    Output {
        name: "horizontal",
        match_: Match::Model("VG270U P"),
        position: (0, 240),
        mode: (2560, 1440, 143.995),
        scale: Some(1.0),
        transform: Some(Transform::None),
        // if this does not work try VARIANT_2
        vrr: Some(VrrMode::VARIANT_3),
        workspaces: &["1", "2", "3", "4", "5"],
    },
    Output {
        name: "vertical",
        match_: Match::Model("BenQ GL2480"),
        position: (2560, 0),
        mode: (1920, 1080, 60.0),
        scale: Some(1.0),
        transform: Some(Transform::Rotate90),
        vrr: None,
        workspaces: &["6", "7", "8", "9", "0"],
    },
];

impl Output {
    fn matches(&self, connector: Connector) -> bool {
        match self.match_ {
            Match::Connector(name) => connector.name() == name,
            Match::Model(model) => connector.model() == model,
        }
    }

    fn connector(&self) -> Option<Connector> {
        connectors()
            .into_iter()
            .find(|&c| c.connected() && self.matches(c))
    }

    fn apply(&self, connector: Connector) {
        let (x, y) = self.position;
        connector.set_position(x, y);

        let (width, height, hz) = self.mode;
        connector.set_mode(width, height, Some((hz * 1000.0).round() as u32));

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

/// The connector the named output is currently on, or one that does not exist
/// if it is not connected.
pub fn connector(name: &str) -> Connector {
    OUTPUTS
        .iter()
        .find(|output| output.name == name)
        .and_then(Output::connector)
        .unwrap_or(Connector(0))
}

/// Points every workspace at the first output in [`OUTPUTS`] that lists it and
/// is connected, which is the order the toml side spells out per workspace.
/// Assigning back to front leaves the earliest output with the last word.
///
/// `disconnected` is the connector of a disconnect event: the compositor still
/// reports it as connected while it hands the event to this configuration.
fn assign_initial_connectors(disconnected: Option<Connector>) {
    for output in OUTPUTS.iter().rev() {
        let Some(connector) = output.connector() else {
            continue;
        };
        if Some(connector) == disconnected {
            continue;
        }
        for workspace in output.workspaces {
            get_workspace(workspace).set_initial_connector(Some(connector));
        }
    }
}

pub fn setup() {
    on_connector_connected(|connector| {
        if let Some(output) = OUTPUTS.iter().find(|output| output.matches(connector)) {
            output.apply(connector);
        }
        assign_initial_connectors(None);
    });
    on_connector_disconnected(|connector| assign_initial_connectors(Some(connector)));
}
