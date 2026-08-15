{
  config,
  lib,
  pkgs,
  ...
}:
{
  programs.prismlauncher = {
    enable = true;
    package =
      with pkgs;
      (prismlauncher.override {
        additionalLibs = [
          libxtst
          libxkbcommon
          libxt
          libxinerama
        ];
        additionalPrograms = [
          ninjabrain-bot
        ];
        jdks = [
          temurin-bin-21
          temurin-bin-25
        ];
      });

    settings =
      let
        fonts = config.stylix.fonts;
      in
      {
        AutoCloseConsole = false;
        AutomaticJavaDownload = false;
        AutomaticJavaSwitch = true;
        CloseAfterLaunch = false;
        EnableFeralGamemode = true;
        EnableMangoHud = false;
        Env = builtins.toJSON (
          builtins.toJSON {
            LD_PRELOAD = "${lib.getLib pkgs.jemalloc}/lib/libjemalloc.so.2";
          }
        );
        IgnoreJavaCompatibility = true;
        JavaPath = lib.getExe pkgs.temurin-bin-21;
        JvmArgs = "-XX:+UseZGC -XX:+AlwaysPreTouch -Dgraal.TuneInlinerExploration=1 -XX:NmethodSweepActivity=1";
        MaxMemAlloc = 12032;
        MinMemAlloc = 1024;
        PermGen = 128;
        ShowConsole = false;
        ShowConsoleOnError = true;
        ShowGameTime = true;
        ShowGameTimeWithoutDays = true;
        ShowGlobalGameTime = true;
        UseNativeGLFW = true;
        UseNativeOpenAL = false;

        # remove when https://github.com/nix-community/stylix/pull/2335 is merged
        ApplicationTheme = "stylix";
        ConsoleFont = fonts.monospace.name;
        ConsoleFontSize = fonts.sizes.terminal;
      };

    # remove when https://github.com/nix-community/stylix/pull/2335 is merged
    themes.stylix.theme = with config.lib.stylix.colors.withHashtag; {
      name = "Stylix";
      widgets = "Fusion";

      colors = {
        AlternateBase = base01;
        Base = base00;
        BrightText = base08;
        Button = base01;
        ButtonText = base05;
        Highlight = base02;
        HighlightedText = base05;
        Link = base0D;
        Text = base05;
        ToolTipBase = base00;
        ToolTipText = base05;
        Window = base00;
        WindowText = base05;
        fadeAmount = 0.5;
        fadeColor = base02;
      };
      logColors = {
        Debug = base0B;
        DebugHighlight = base03;
        Error = base08;
        ErrorHighlight = base03;
        Fatal = base08;
        FatalHighlight = base00;
        Launcher = base0D;
        LauncherHighlight = base03;
        Message = base05;
        MessageHighlight = base02;
        Warning = base0A;
        WarningHighlight = base03;
      };
    };
  };

  preservation.preserveAt.state-dir.directories = [
    ".local/share/PrismLauncher"
  ];
}
