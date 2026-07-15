use jay_config::config;

mod bar;
mod behavior;
mod clients;
mod env;
mod inputs;
mod outputs;
mod power;
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
    bar::setup();
}

config!(configure);
