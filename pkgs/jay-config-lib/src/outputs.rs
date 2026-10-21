//! find names and models of outputs with `jay randr`

use jay_config::{
    get_workspace,
    video::{
        Connector, Transform, VrrMode, connectors, on_connector_connected,
        on_connector_disconnected,
    },
};

pub enum Match {
    Connector(&'static str),
    Model(&'static str),
}

pub const LAPTOP_PANEL: Match = Match::Connector("eDP-1");
pub const BEAMER: Match = Match::Model("EPSON PJ");

impl Match {
    fn matches(&self, connector: Connector) -> bool {
        match *self {
            Match::Connector(name) => connector.name() == name,
            Match::Model(model) => connector.model() == model,
        }
    }

    fn find(&self) -> Option<Connector> {
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
    match_: Match,
    position: (i32, i32),
    /// width, height, refresh rate in Hz; must be one the output supports
    mode: (i32, i32, f64),
    scale: Option<f64>,
    transform: Option<Transform>,
    vrr: Option<VrrMode>,
    /// the workspaces that start out here, unless an earlier output has them
    workspaces: &'static [&'static str],
}

const OUTPUTS: &[Output] = &[
    Output {
        match_: LAPTOP_PANEL,
        position: (0, 0),
        mode: (1920, 1080, 60.049),
        scale: None,
        transform: None,
        vrr: None,
        workspaces: &["1", "2", "3", "4", "5"],
    },
    Output {
        // TODO: automatically spawn wl-mirror
        match_: BEAMER,
        position: (0, 1080),
        mode: (1920, 1080, 60.0),
        scale: None,
        transform: None,
        vrr: None,
        workspaces: &["0"],
    },
    Output {
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

/// `disconnected` is still reported as connected while its event is handled.
fn assign_workspaces(disconnected: Option<Connector>) {
    // back to front, so that earlier outputs win
    for output in OUTPUTS.iter().rev() {
        if let Some(connector) = output.match_.find()
            && Some(connector) != disconnected
        {
            for workspace in output.workspaces {
                get_workspace(workspace).set_initial_connector(Some(connector));
            }
        }
    }
}

pub fn setup() {
    on_connector_connected(|connector| {
        if let Some(output) = OUTPUTS.iter().find(|o| o.match_.matches(connector)) {
            output.apply(connector);
        }
        assign_workspaces(None);
    });
    on_connector_disconnected(|connector| assign_workspaces(Some(connector)));
}
