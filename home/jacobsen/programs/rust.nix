{ pkgs, ... }:
{
  # fallback toolchain (overwritten by dev shell toolchains)
  packages = with pkgs; [
    cargo
    clippy
    rust-analyzer
    rustc
    rustfmt
  ];
}
