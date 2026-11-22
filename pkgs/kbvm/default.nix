{
  lib,
  stdenv,
  autoPatchelfHook,
  inputs,
  installShellFiles,
  libx11,
  makeWrapper,
  pkg-config,
  pkgs,
  wayland,
  xkeyboard_config,
}:
let
  craneLib = inputs.crane.mkLib pkgs;

  common = {
    pname = "kbvm";
    version = (lib.importTOML "${inputs.kbvm}/kbvm-cli/Cargo.toml").package.version;
    src = inputs.kbvm;
    strictDeps = true;
    cargoExtraArgs = "--locked -p kbvm-cli";
    nativeBuildInputs = [ pkg-config ];
    buildInputs = [ xkeyboard_config ];
    doCheck = false;
  };
in
craneLib.buildPackage (
  common
  // {
    cargoArtifacts = craneLib.buildDepsOnly common;
    nativeBuildInputs = common.nativeBuildInputs ++ [
      autoPatchelfHook
      installShellFiles
      makeWrapper
    ];
    buildInputs = common.buildInputs ++ [ stdenv.cc.cc.lib ];
    runtimeDependencies = [ wayland ];
    postInstall = ''
      installShellCompletion --cmd kbvm \
        --bash <($out/bin/kbvm generate-completion bash) \
        --fish <($out/bin/kbvm generate-completion fish) \
        --zsh <($out/bin/kbvm generate-completion zsh)
      wrapProgram $out/bin/kbvm \
        --set-default XLOCALEDIR ${libx11.out}/share/X11/locale
    '';
    meta = {
      description = "A rust implementation of XKB";
      homepage = "https://github.com/mahkoh/kbvm";
      license = lib.licenses.gpl3Only;
      mainProgram = "kbvm";
      platforms = lib.platforms.linux;
    };
  }
)
