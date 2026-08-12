//! The wayland protocols each client is allowed to use.

use jay_config::client::{
    CC_DATA_CONTROL, CC_FOREIGN_TOPLEVEL_MANAGER, CC_IDLE_NOTIFIER, CC_LAYER_SHELL, CC_SCREENCOPY,
    CC_SESSION_LOCK, CC_WORKSPACE_MANAGER, ClientCapabilities, ClientCriterion,
};

pub fn setup() {
    // Matched by executable basename rather than by full (Nix store) path, so
    // this keeps working across rebuilds without needing store paths in the
    // config. Nix wraps some packages (e.g. via `wrapProgram`): the wrapper
    // script `exec`s into the real binary renamed to `.<name>-wrapped`, and
    // that's what ends up as the client's exe, not the plain `<name>`.
    let capabilities: [(&str, ClientCapabilities); 9] = [
        ("grim", CC_SCREENCOPY),
        ("swayidle", CC_IDLE_NOTIFIER),
        ("swaylock", CC_LAYER_SHELL | CC_SESSION_LOCK),
        ("swaync", CC_LAYER_SHELL),
        (
            "waybar",
            CC_FOREIGN_TOPLEVEL_MANAGER | CC_LAYER_SHELL | CC_WORKSPACE_MANAGER,
        ),
        ("wayland-pipewire-idle-inhibit", CC_LAYER_SHELL),
        ("wl-copy|wl-paste", CC_DATA_CONTROL),
        ("wl-clip-persist", CC_DATA_CONTROL),
        ("wl-mirror", CC_SCREENCOPY),
    ];

    for (pattern, capabilities) in capabilities {
        let pattern = format!(r"/\.?({pattern})(-wrapped)?$");
        ClientCriterion::ExeRegex(&pattern)
            .to_matcher()
            .set_capabilities(capabilities);
    }
}
