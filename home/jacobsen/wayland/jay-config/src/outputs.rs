use jay_config::video::{Connector, Transform, VrrMode, connectors, get_connector, on_connector_connected};

fn hz(refresh_hz: f64) -> u32 {
    (refresh_hz * 1000.0).round() as u32
}

/// The laptop's built-in display.
pub fn laptop_integrated() -> Connector {
    get_connector("eDP-1")
}

fn by_model(model: &str) -> Connector {
    for c in connectors() {
        if c.connected() && c.model() == model {
            return c;
        }
    }
    Connector(0)
}

/// The beamer/projector.
pub fn beamer() -> Connector {
    by_model("EPSON PJ")
}

/// The desk monitor mounted in landscape orientation.
pub fn horizontal() -> Connector {
    by_model("VG270U P")
}

/// The desk monitor mounted in portrait orientation.
pub fn vertical() -> Connector {
    by_model("BenQ GL2480")
}

pub fn setup() {
    on_connector_connected(|con| match (con.name().as_str(), con.model().as_str()) {
        ("eDP-1", _) => {
            // laptop integrated display
            con.set_position(0, 0);
            con.set_mode(1920, 1080, Some(hz(60.049)));
        }
        (_, "EPSON PJ") => {
            // external beamer
            con.set_position(0, 1080);
            con.set_mode(1920, 1080, Some(hz(60.0)));
        }
        (_, "VG270U P") => {
            // horizontal display
            con.set_position(0, 240);
            con.set_scale(1.0);
            con.set_mode(2560, 1440, Some(hz(143.995)));
            con.set_transform(Transform::None);
            con.set_vrr_mode(VrrMode::VARIANT_3);
        }
        (_, "BenQ GL2480") => {
            // vertical display
            con.set_position(2560, 0);
            con.set_scale(1.0);
            con.set_mode(1920, 1080, Some(hz(60.0)));
            con.set_transform(Transform::Rotate90);
        }
        _ => {}
    });
}
