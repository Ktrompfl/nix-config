use jay_config::client::{
    CC_DATA_CONTROL, CC_FOREIGN_TOPLEVEL_MANAGER, CC_LAYER_SHELL, CC_SCREENCOPY, CC_SESSION_LOCK,
    CC_WORKSPACE_MANAGER, ClientCapabilities, ClientCriterion,
};

// Matched by executable basename rather than by full (Nix store) path, so
// this keeps working across rebuilds without needing store paths generated
// from Nix. The jay home-manager module makes sure these are all installed.
//
// Nix wraps some packages (e.g. via `wrapProgram`): the wrapper script
// `exec`s into the real binary renamed to `.<name>-wrapped`, and that's what
// ends up as the client's exe, not the plain `<name>`. Match both forms.
fn bind(name_pattern: &str, capabilities: ClientCapabilities) {
    let pattern = format!(r"/\.?({name_pattern})(-wrapped)?$");
    ClientCriterion::ExeRegex(&pattern).to_matcher().set_capabilities(capabilities);
}

pub fn setup() {
    bind("swayidle", CC_IDLE_NOTIFIER);

    bind("swaylock", CC_LAYER_SHELL | CC_SESSION_LOCK);

    bind("swaync", CC_LAYER_SHELL);

    bind("waybar", CC_FOREIGN_TOPLEVEL_MANAGER | CC_LAYER_SHELL | CC_WORKSPACE_MANAGER);

    bind("grim", CC_SCREENCOPY);

    bind("wl-mirror", CC_SCREENCOPY);

    bind("wayland-pipewire-idle-inhibit", CC_LAYER_SHELL);

    bind("wl-copy|wl-paste", CC_DATA_CONTROL);

    bind("wl-clip-persist", CC_DATA_CONTROL);
}
