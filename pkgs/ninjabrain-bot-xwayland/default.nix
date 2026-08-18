{
  lib,
  makeWrapper,
  ninjabrain-bot,
  python3Packages,
  runCommand,
  wl-clipboard,
  writers,
  xclip,
  xwayland,
}:
let
  script = writers.writePython3Bin "ninjabrain-bot-xwayland" {
    libraries = [ python3Packages.xlib ];
    flakeIgnore = [ "E501" ];
  } (builtins.readFile ./main.py);
in
runCommand "ninjabrain-bot-xwayland"
  {
    nativeBuildInputs = [ makeWrapper ];

    meta = {
      description = "Ninjabrain Bot (Xwayland)";
      mainProgram = "ninjabrain-bot-xwayland";
      inherit (ninjabrain-bot.meta) license;
    };
  }
  ''
    makeWrapper ${lib.getExe script} $out/bin/ninjabrain-bot-xwayland \
      --prefix PATH : ${
        lib.makeBinPath [
          ninjabrain-bot
          wl-clipboard
          xclip
          xwayland
        ]
      }
  ''
