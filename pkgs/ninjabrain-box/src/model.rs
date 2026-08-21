//! What the bot knows, and everything the panel shows that is derived from it.
//!
//! The HTTP API reports the state the bot's own main panel is drawn from, but
//! only the parts it cannot recompute: the panel's nether column, distances
//! and travel angles are all arithmetic on the chunk and the player, so they
//! are done here rather than asked for.

use serde::Deserialize;

/// A stronghold the bot considers possible.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Prediction {
    pub certainty: f64,
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub overworld_distance: f64,
}

/// One measured ender eye throw.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Throw {
    #[serde(rename = "xInOverworld")]
    pub x: f64,
    #[serde(rename = "zInOverworld")]
    pub z: f64,
    pub angle: f64,
    /// How far the angle was nudged from what was measured.
    #[serde(default)]
    pub correction: f64,
    /// The same nudge counted in the increments the adjustment is made in --
    /// what you actually press, rather than what it comes to in degrees.
    #[serde(default)]
    pub correction_increments: i32,
    #[serde(default)]
    pub error: f64,
}

/// Where the last F3+C put the player.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Player {
    #[serde(rename = "xInOverworld")]
    pub x: Option<f64>,
    #[serde(rename = "zInOverworld")]
    pub z: Option<f64>,
    pub horizontal_angle: Option<f64>,
    #[serde(default)]
    pub is_in_nether: bool,
}

/// The `/api/v1/stronghold` response.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stronghold {
    #[serde(default)]
    pub result_type: String,
    #[serde(default)]
    pub predictions: Vec<Prediction>,
    #[serde(default)]
    pub eye_throws: Vec<Throw>,
    #[serde(default)]
    pub player_position: Player,
}

impl Stronghold {
    /// The line the bot's own panel shows in place of a table.
    pub fn placeholder(&self) -> Option<&'static str> {
        match self.result_type.as_str() {
            "FAILED" => Some("Could not determine the stronghold chunk."),
            _ if self.predictions.is_empty() => Some("Waiting for F3+C..."),
            _ => None,
        }
    }
}

impl Prediction {
    /// Chunk coordinates, as the bot's `chunk` display type shows them.
    pub fn chunk(&self) -> (i32, i32) {
        (self.chunk_x, self.chunk_z)
    }

    /// The block the stronghold is expected in, which is the chunk's centre.
    pub fn block(&self) -> (i32, i32) {
        (self.chunk_x * 16 + 8, self.chunk_z * 16 + 8)
    }

    /// The nether coordinates above the stronghold: overworld blocks over
    /// eight, which for a whole chunk is just the chunk doubled.
    pub fn nether(&self) -> (i32, i32) {
        (self.chunk_x * 2, self.chunk_z * 2)
    }

    /// Distance in whichever dimension the player is standing in.
    pub fn distance(&self, player: &Player) -> f64 {
        if player.is_in_nether {
            self.overworld_distance / 8.0
        } else {
            self.overworld_distance
        }
    }

    /// The yaw to face to walk towards this stronghold, in Minecraft's
    /// convention: zero looks along +z, and yaw grows towards -x.
    pub fn travel_angle(&self, player: &Player) -> Option<f64> {
        let (px, pz) = (player.x?, player.z?);
        let (bx, bz) = self.block();
        let (dx, dz) = (bx as f64 - px, bz as f64 - pz);
        Some(normalize_angle(-dx.atan2(dz).to_degrees()))
    }

    /// How far the player has to turn to face it.
    pub fn travel_angle_delta(&self, player: &Player) -> Option<f64> {
        Some(normalize_angle(
            self.travel_angle(player)? - player.horizontal_angle?,
        ))
    }
}

/// Folds an angle into (-180, 180].
pub fn normalize_angle(mut degrees: f64) -> f64 {
    while degrees <= -180.0 {
        degrees += 360.0;
    }
    while degrees > 180.0 {
        degrees -= 360.0;
    }
    degrees
}
