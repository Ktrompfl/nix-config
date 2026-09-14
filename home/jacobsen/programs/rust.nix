{
  config,
  osConfig,
  pkgs,
  ...
}:
let
  stateHome = "${osConfig.preservation.preserveAt.state-dir.persistentStoragePath}${config.directory}";
in
{
  environment.sessionVariables = {
    CARGO_HOME = "${stateHome}/.local/share/cargo";
    RUSTUP_HOME = "${stateHome}/.local/share/rustup";
  };

  # fallback toolchain (overwritten by dev shell toolchains)
  packages = with pkgs; [
    cargo
    clippy
    rust-analyzer
    rustc
    rustfmt
  ];
}
