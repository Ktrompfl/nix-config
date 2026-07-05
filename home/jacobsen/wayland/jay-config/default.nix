{
  lib,
  rustPlatform,
  inputs,
  extraEnv ? { },
}:
rustPlatform.buildRustPackage {
  pname = "jay-config-lib";
  version = "0.1.0";

  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./src
    ];
  };

  cargoLock.lockFile = ./Cargo.lock;

  # `jay-config` is not fetched from crates.io: we compile against the exact
  # version vendored in the `jay` flake input so that the config.so is always
  # ABI-compatible with the jay binary it will be loaded into.
  postPatch = ''
    mkdir -p vendor
    cp -r ${inputs.jay}/jay-config vendor/jay-config
    chmod -R u+w vendor/jay-config

    cat > src/generated.rs <<EOF
    #![allow(dead_code)]
    ${lib.concatStrings (
      lib.mapAttrsToList (name: value: ''
        pub const ${name}: &str = ${builtins.toJSON (toString value)};
      '') extraEnv
    )}
    EOF
  '';

  # This builds a config.so plugin that Jay loads via dlopen, not a standalone
  # executable, so there is nothing for `cargo install` to place in $out/bin.
  installPhase = ''
    runHook preInstall
    install -Dm755 "$(find target -name libjay_config_lib.so -not -path '*/deps/*')" $out/lib/config.so
    runHook postInstall
  '';

  meta = {
    description = "jay shared library configuration";
    platforms = lib.platforms.linux;
  };
}
