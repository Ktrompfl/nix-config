{ config, pkgs, ... }:
{
  environment.sessionVariables = {
    CARGO_HOME = "${config.xdg.data.directory}/cargo";
    RUSTUP_HOME = "${config.xdg.data.directory}/rustup";
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
