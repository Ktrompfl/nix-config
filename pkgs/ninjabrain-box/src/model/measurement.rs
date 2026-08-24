//! What the game reports when the player asks it where they are.
//!
//! This is the game's format, not any calculator's: an F3+C line is what
//! Minecraft puts on the clipboard, and every stronghold calculator that
//! exists takes its input from one. It is kept verbatim as well as parsed,
//! because a calculator that has to be told about a measurement again should
//! be told exactly what the game said, not a reconstruction.

/// Which world a measurement was taken in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dimension {
    Overworld,
    Nether,
    End,
}

/// One F3+C line, kept exactly as the game wrote it.
#[derive(Clone, Debug)]
pub struct Measurement {
    pub x: f64,
    pub z: f64,
    /// Where the player was looking, left to right, in degrees.
    pub yaw: f64,
    /// And up and down. Eyes thrown below the horizon are not throws.
    pub pitch: f64,
    pub dimension: Dimension,
    /// The line itself.
    pub line: String,
}

/// Two measurements are the same when they are the same line. Whether they
/// are the same *observation*, which tolerates a differently written line, is
/// [`Measurement::is`].
impl PartialEq for Measurement {
    fn eq(&self, other: &Measurement) -> bool {
        self.line == other.line
    }
}

impl Eq for Measurement {}

impl Measurement {
    /// Parses the line Minecraft's F3+C puts on the clipboard.
    ///
    /// ```text
    /// /execute in minecraft:overworld run tp @s 123.45 68.00 -456.78 -12.34 -31.00
    /// ```
    ///
    /// Eleven space-separated fields, which is what the game writes and what
    /// every calculator expects to be handed.
    pub fn parse(text: &str) -> Option<Measurement> {
        let line = text.trim();
        let fields: Vec<&str> = line.split(' ').collect();
        let ["/execute", "in", world, "run", "tp", "@s", x, y, z, yaw, pitch] = fields[..] else {
            return None;
        };
        let dimension = match world {
            "minecraft:overworld" => Dimension::Overworld,
            "minecraft:the_nether" => Dimension::Nether,
            "minecraft:the_end" => Dimension::End,
            _ => return None,
        };
        // `y` is parsed only to insist the line is well formed; nothing needs
        // to keep it, and the line itself is kept whole regardless.
        let [x, _, z, yaw, pitch] = [x, y, z, yaw, pitch].map(|field| field.parse::<f64>().ok());
        Some(Measurement {
            x: x?,
            z: z?,
            yaw: yaw?,
            pitch: pitch?,
            dimension,
            line: line.to_owned(),
        })
    }

    /// Whether this measurement could be an eye throw at all. Throws are made
    /// in the overworld, looking at or above the horizon -- in Minecraft's
    /// convention a positive pitch is looking down.
    pub fn is_throwable(&self) -> bool {
        self.dimension == Dimension::Overworld && self.pitch <= 0.0
    }

    /// Whether two measurements are the same observation. Compared on what the
    /// game measured rather than on the text, so that a line differing only in
    /// spacing is still recognised.
    pub fn is(&self, other: &Measurement) -> bool {
        const CLOSE: f64 = 1e-6;
        self.dimension == other.dimension
            && (self.x - other.x).abs() < CLOSE
            && (self.z - other.z).abs() < CLOSE
            && (self.yaw - other.yaw).abs() < CLOSE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LINE: &str =
        "/execute in minecraft:overworld run tp @s 100.50 68.00 -200.25 -45.60 -31.00";

    #[test]
    fn parses_what_the_game_writes() {
        let measurement = Measurement::parse(LINE).expect("a measurement");
        assert_eq!(measurement.x, 100.50);
        assert_eq!(measurement.z, -200.25);
        assert_eq!(measurement.yaw, -45.60);
        assert_eq!(measurement.pitch, -31.00);
        assert_eq!(measurement.dimension, Dimension::Overworld);
        assert_eq!(measurement.line, LINE);
        assert!(measurement.is_throwable());
    }

    #[test]
    fn refuses_what_is_not_one() {
        for text in [
            "hello",
            "/execute in minecraft:overworld run tp @s 1 2 3",
            "/execute in minecraft:the_void run tp @s 1 2 3 4 5",
            "/execute in minecraft:overworld run tp @s 1 2 3 north 5",
        ] {
            assert!(Measurement::parse(text).is_none(), "{text}");
        }
    }

    /// An eye thrown at the ground is not a throw, and neither is one thrown
    /// in the nether -- both are positions, and the bot treats them as such.
    #[test]
    fn knows_what_cannot_be_a_throw() {
        let looking_down = LINE.replace("-31.00", "12.00");
        assert!(!Measurement::parse(&looking_down).unwrap().is_throwable());

        let nether = LINE.replace("overworld", "the_nether");
        assert!(!Measurement::parse(&nether).unwrap().is_throwable());
    }
}
