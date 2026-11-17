# the development shell for all rust projects under pkgs/ (`nix develop .#rust`,
# or direnv through pkgs/.envrc); the test tools run getaway nested in a headless sway
{ inputs, pkgs }:
let
  rustBin = inputs.rust-overlay.lib.mkRustBin { } pkgs;
in
pkgs.mkShell {
  # the same jay source and lockfile the package build uses
  shellHook = ''
    lib=$(git rev-parse --show-toplevel)/pkgs/jay-config-lib
    ln -sfn ${pkgs.jay.src} "$lib/jay"
    install -m 0644 ${pkgs.jay.src}/Cargo.lock "$lib/Cargo.lock"
  '';

  packages = [
    (rustBin.stable.latest.default.override {
      extensions = [
        "rust-src"
        "rustfmt"
        "clippy"
        "rust-analyzer"
      ];
    })
  ]
  ++ pkgs.getaway.libraries
  ++ (with pkgs; [
    mold
    shaderc
    pkg-config

    sway-unwrapped
    grim
    wtype
    foot
    wayland-utils
    wl-clipboard
    wlrctl
    wlr-randr
    xwayland-satellite
    xwayland
    vulkan-tools
    mesa-demos
  ]);

  RUSTFLAGS = "-C link-arg=-fuse-ld=mold";
  LD_LIBRARY_PATH = "${pkgs.vulkan-loader}/lib";
  XKB_CONFIG_ROOT = "${pkgs.xkeyboard-config}/share/X11/xkb";
  PROTOCOL_DIR = pkgs.getaway.protocols;
  XCURSOR_PATH = "${pkgs.adwaita-icon-theme}/share/icons";
  XCURSOR_THEME = "Adwaita";
}
