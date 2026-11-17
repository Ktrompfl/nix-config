mod config;

use wm::{Bindings, Shell};

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("--probe-kms") => server::probe_kms(config::server()),
        _ => {
            let mut bindings = Bindings::default();
            config::bindings(&mut bindings);
            let shell = Shell::new(config::Wm::new(config::theme()), bindings);
            server::run(Box::new(shell), config::server(), &args)
        }
    };
    if let Err(e) = result {
        log::error!("{e}");
        std::process::exit(1);
    }
}
