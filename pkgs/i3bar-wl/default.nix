{
  lib,
  pkgs,
  inputs,
}:
let
  craneLib = inputs.crane.mkLib pkgs;

  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
    ];
  };

  common = {
    pname = "i3bar-wl";
    version = "0.1.0";
    inherit src;
    strictDeps = true;
    cargoExtraArgs = "--locked";
    nativeBuildInputs = [ pkgs.pkg-config ];
    buildInputs = with pkgs; [
      cairo
      glib
      pango
    ];
    doCheck = false;
  };
in
craneLib.buildPackage (
  common
  // {
    cargoArtifacts = craneLib.buildDepsOnly common;
    meta = {
      description = "Status bar with i3 input protocol for Wayland";
      mainProgram = "i3bar-wl";
      platforms = lib.platforms.linux;
    };
  }
)
