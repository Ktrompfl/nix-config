use jay_config::client::{
    CC_DATA_CONTROL, CC_FOREIGN_TOPLEVEL_MANAGER, CC_LAYER_SHELL, CC_SCREENCOPY, CC_WORKSPACE_MANAGER,
    ClientCriterion,
};

pub fn setup() {
    ClientCriterion::ExeRegex("/swaync$")
        .to_matcher()
        .set_capabilities(CC_LAYER_SHELL);

    ClientCriterion::ExeRegex("/waybar$")
        .to_matcher()
        .set_capabilities(CC_FOREIGN_TOPLEVEL_MANAGER | CC_LAYER_SHELL | CC_WORKSPACE_MANAGER);

    ClientCriterion::ExeRegex("/grim$")
        .to_matcher()
        .set_capabilities(CC_SCREENCOPY);

    ClientCriterion::ExeRegex("/wl-mirror$")
        .to_matcher()
        .set_capabilities(CC_SCREENCOPY);

    ClientCriterion::ExeRegex("/wayland-pipewire-idle-inhibit$")
        .to_matcher()
        .set_capabilities(CC_LAYER_SHELL);

    ClientCriterion::ExeRegex("/(wl-copy|wl-paste)$")
        .to_matcher()
        .set_capabilities(CC_DATA_CONTROL);

    ClientCriterion::ExeRegex("/wl-clip-persist$")
        .to_matcher()
        .set_capabilities(CC_DATA_CONTROL);
}
