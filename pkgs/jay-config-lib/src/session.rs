use jay_config::video::on_graphics_initialized;

use crate::exec;

pub fn setup() {
    on_graphics_initialized(|| {
        exec("systemctl", &["--user", "start", "jay-session.target"]);
    });
}
