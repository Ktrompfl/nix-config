{
  lib,
  pkgs,
  inputs,
  jay,
}:
let
  craneLib = inputs.crane.mkLib pkgs;

  linkJay = ''
    ln -s ${jay.src} jay
    install -m 0644 ${jay.src}/Cargo.lock Cargo.lock
  '';

  commonArgs = {
    inherit (jay) version;
    pname = "jay-config-lib";
    src = lib.fileset.toSource {
      root = ./.;
      fileset = lib.fileset.unions [
        ./Cargo.toml
        ./src
      ];
    };
    strictDeps = true;
    doCheck = false;

    cargoVendorDir = craneLib.vendorCargoDeps { cargoLock = "${jay.src}/Cargo.lock"; };
    postPatch = linkJay;
    # jay's lockfile lacks an entry for this crate, which cargo adds offline from
    # the vendored dependencies; `--locked` would refuse to.
    cargoExtraArgs = "--offline";
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
