use jay_config::client::{
    CC_DATA_CONTROL, CC_FOREIGN_TOPLEVEL_MANAGER, CC_IDLE_NOTIFIER, CC_LAYER_SHELL, CC_SCREENCOPY,
    CC_SESSION_LOCK, ClientCriterion,
};

pub fn setup() {
    let capabilities = [
        ("grim", CC_SCREENCOPY),
        (
            "ninjabrain-box",
            CC_DATA_CONTROL | CC_FOREIGN_TOPLEVEL_MANAGER | CC_LAYER_SHELL,
        ),
        ("swayidle", CC_IDLE_NOTIFIER),
        ("swaylock", CC_LAYER_SHELL | CC_SESSION_LOCK),
        ("swaync", CC_LAYER_SHELL),
        ("wayland-pipewire-idle-inhibit", CC_LAYER_SHELL),
        ("wl-copy|wl-paste", CC_DATA_CONTROL),
        ("wl-clip-persist", CC_DATA_CONTROL),
        ("wl-mirror", CC_SCREENCOPY),
    ];

    // by basename, which survives rebuilds; nix wrappers exec `.<name>-wrapped`
    for (name, capabilities) in capabilities {
        ClientCriterion::ExeRegex(&format!(r"/\.?({name})(-wrapped)?$"))
            .to_matcher()
            .set_capabilities(capabilities);
    }
}
