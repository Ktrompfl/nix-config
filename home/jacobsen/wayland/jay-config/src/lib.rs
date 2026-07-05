use jay_config::config;

mod behavior;
mod clients;
mod env;
mod generated;
mod inputs;
mod modes;
mod outputs;
mod shortcuts;
mod theme;
mod windows;

fn configure() {
    env::setup();
    theme::setup();

    inputs::setup();
    outputs::setup();

    shortcuts::setup();
    windows::setup();
    clients::setup();
    behavior::setup();
}

config!(configure);
