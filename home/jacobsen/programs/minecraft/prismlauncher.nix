{
  config,
  lib,
  pkgs,
  ...
}:
let
  fonts = config.theme.fonts;
in
{
  packages = [
    (pkgs.prismlauncher.override {
      additionalLibs = with pkgs; [
        libxtst
        libxkbcommon
        libxt
        libxinerama
      ];
      jdks = with pkgs; [
        temurin-bin-21
      ];
    })
  ];

  xdg.data.files."PrismLauncher/prismlauncher.cfg" = {
    type = "copy";
    clobber = true;
    generator = lib.generators.toINI { };
    value.General = {

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

      ApplicationTheme = "tinted";
      ConsoleFont = fonts.monospace.name;
      ConsoleFontSize = fonts.sizes.terminal;
    };
  };

  xdg.data.files."PrismLauncher/themes/tinted/theme.json" = {
    generator = (pkgs.formats.json { }).generate "prismlauncher-theme.json";
    value = with config.theme.colors.withHashtag; {

      name = "Tinted";
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
