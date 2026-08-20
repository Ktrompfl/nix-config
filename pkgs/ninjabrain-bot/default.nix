{
  imagemagick,
  jdk,
  lib,
  makeWrapper,
  ninjabrain-bot,
  runCommand,
  stdenvNoCC,
  wl-clipboard,
  writers,
}:
let
  jar = "${ninjabrain-bot}/share/java/ninjabrain-bot.jar";

  # Compiled against the bot itself: two of the three classes sit in its
  # packages to reach seams it only exposes to them, so a rename upstream is a
  # build failure here rather than a bot that quietly stops responding.
  agent = stdenvNoCC.mkDerivation {
    name = "ninjabrain-bot-ipc.jar";
    src = lib.sources.sourceFilesBySuffices ./. [ ".java" ];
    nativeBuildInputs = [ jdk ];

    buildPhase = ''
      runHook preBuild
      javac -Xlint:all -Werror -classpath ${jar} -d classes *.java
      # Premain-Class is the whole of what makes a jar an agent.
      printf 'Premain-Class: ninjabrainbot.ipc.Agent\n' > manifest.mf
      jar --create --file ninjabrain-bot-ipc.jar --manifest manifest.mf -C classes .
      runHook postBuild
    '';

    installPhase = ''
      runHook preInstall
      install -Dm644 ninjabrain-bot-ipc.jar $out
      runHook postInstall
    '';
  };

  script =
    writers.writePython3Bin "ninjabrain-bot"
      {
        flakeIgnore = [ "E501" ];
      }
      (
        builtins.replaceStrings [ "@ninjabrain-bot@" ] [ (lib.getExe ninjabrain-bot) ] (
          builtins.readFile ./main.py
        )
      );
in
runCommand "ninjabrain-bot-${ninjabrain-bot.version}"
  {
    inherit (ninjabrain-bot) pname version;

    nativeBuildInputs = [
      imagemagick
      makeWrapper
    ];

    passthru = ninjabrain-bot.passthru or { } // {
      unwrapped = ninjabrain-bot;
    };

    meta = ninjabrain-bot.meta // {
      description = "${ninjabrain-bot.meta.description}, driveable from outside the program";
    };
  }
  ''
    makeWrapper ${lib.getExe script} $out/bin/ninjabrain-bot \
      --set JDK_JAVA_OPTIONS "-javaagent:${agent}" \
      --prefix PATH : ${lib.makeBinPath [ wl-clipboard ]}

    mkdir -p $out/share
    ln -s ${ninjabrain-bot}/share/java $out/share/java
    install -Dm644 ${ninjabrain-bot}/share/applications/ninjabrain-bot.desktop \
      $out/share/applications/ninjabrain-bot.desktop

    # The bot ships one 640x640 icon, a size hicolor does not declare, so
    # nothing looking an icon up the documented way ever finds it.
    install -Dm644 ${ninjabrain-bot}/share/icons/hicolor/640x640/apps/ninjabrain-bot.png \
      $out/share/icons/hicolor/640x640/apps/ninjabrain-bot.png
    for size in 48 64 128 256 512; do
      dir=$out/share/icons/hicolor/''${size}x''${size}/apps
      mkdir -p $dir
      magick ${ninjabrain-bot}/share/icons/hicolor/640x640/apps/ninjabrain-bot.png \
        -resize ''${size}x''${size} $dir/ninjabrain-bot.png
    done
  ''
