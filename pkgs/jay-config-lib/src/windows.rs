use jay_config::{
    ContainerTarget, RelativeAxis,
    window::{CONTAINER, MatchedWindow, WindowCriterion},
};

pub fn setup() {
    // tile workspace containers on their major axis
    WindowCriterion::All(&[
        WindowCriterion::Types(CONTAINER),
        WindowCriterion::IsWorkspaceContainer,
        WindowCriterion::JustMapped,
    ])
    .bind(|matched: MatchedWindow| {
        matched
            .window()
            .set_container_split_relative(ContainerTarget::Itself, RelativeAxis::Major);
    });
}
