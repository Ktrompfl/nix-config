{
  lib,
  pkgs,
  inputs,
}:
let
  craneLib = inputs.crane.mkLib pkgs;

  common = {
    pname = "kbvm";
    version = (lib.importTOML "${inputs.kbvm}/kbvm-cli/Cargo.toml").package.version;
    src = inputs.kbvm;
    strictDeps = true;
    cargoExtraArgs = "--locked -p kbvm-cli";
    nativeBuildInputs = [ pkgs.pkg-config ];
    buildInputs = [ pkgs.xkeyboard_config ];
    doCheck = false;
  };
in
craneLib.buildPackage (
  common
  // {
    cargoArtifacts = craneLib.buildDepsOnly common;
    nativeBuildInputs = common.nativeBuildInputs ++ [
      pkgs.installShellFiles
      pkgs.makeWrapper
    ];
    postInstall = ''
      installShellCompletion --cmd kbvm \
        --bash <($out/bin/kbvm generate-completion bash) \
        --fish <($out/bin/kbvm generate-completion fish) \
        --zsh <($out/bin/kbvm generate-completion zsh)
      wrapProgram $out/bin/kbvm \
        --set-default XLOCALEDIR ${pkgs.libx11.out}/share/X11/locale
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
