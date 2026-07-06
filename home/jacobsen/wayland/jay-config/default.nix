{
  lib,
  pkgs,
  inputs,
}:
let
  craneLib = inputs.crane.mkLib pkgs;

  commonArgs = {
    src = lib.fileset.toSource {
      root = ./.;
      fileset = lib.fileset.unions [
        ./Cargo.toml
        ./Cargo.lock
        ./src
      ];
    };
    pname = "jay-config-lib";
    version = "0.1.0";
    strictDeps = true;
    doCheck = false;
  };

  cargoArtifacts = craneLib.buildDepsOnly commonArgs;
in
craneLib.buildPackage (
  commonArgs
  // {
    inherit cargoArtifacts;

    postInstall = ''
      mv $out/lib/libjay_config_lib.so $out/lib/config.so
    '';

    meta = {
      description = "jay shared library configuration";
      platforms = lib.platforms.linux;
    };
  }
)
