{
  lib,
  pkgs,
  inputs,
}:
let
  craneLib = inputs.crane.mkLib pkgs;

  # `jay-config` is not fetched from crates.io: we compile against the exact
  # version vendored in the `jay` flake input so that the config.so is always
  # ABI-compatible with the jay binary it will be loaded into. Vendoring it
  # has to happen before crane ever sees the source, since crane's
  # dependency-only build derives its (cached) dummy source straight from
  # Cargo.toml/Cargo.lock on disk rather than through a build-time postPatch.
  src = pkgs.runCommand "jay-config-lib-src" { } ''
    mkdir -p $out
    cp -r ${
      lib.fileset.toSource {
        root = ./.;
        fileset = lib.fileset.unions [
          ./Cargo.toml
          ./Cargo.lock
          ./src
        ];
      }
    }/. $out/
    chmod -R u+w $out

    mkdir -p $out/vendor
    cp -r ${inputs.jay}/jay-config $out/vendor/jay-config
    chmod -R u+w $out/vendor/jay-config
  '';

  commonArgs = {
    inherit src;
    pname = "jay-config-lib";
    version = "0.1.0";
    strictDeps = true;
    doCheck = false;
  };

  # Built from a dummy source containing only Cargo.toml/Cargo.lock, so this
  # is only rebuilt when dependencies actually change, not on every edit to
  # the config itself.
  cargoArtifacts = craneLib.buildDepsOnly commonArgs;
in
craneLib.buildPackage (
  commonArgs
  // {
    inherit cargoArtifacts;

    # This builds a config.so plugin that Jay loads via dlopen, not a
    # standalone executable, so there is nothing for crane to place in
    # $out/bin — install the cdylib ourselves.
    installPhaseCommand = ''
      install -Dm755 "$(find target -name libjay_config_lib.so -not -path '*/deps/*')" $out/lib/config.so
    '';

    meta = {
      description = "jay shared library configuration";
      platforms = lib.platforms.linux;
    };
  }
)
