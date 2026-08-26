{
  config,
  lib,
  osConfig,
  pkgs,
  ...
}:
let
  inherit (config.theme) fonts;
  inherit (config.xdg.data) directory;
  inherit (lib)
    concatMap
    concatStringsSep
    filter
    generators
    getExe
    getLib
    listToAttrs
    nameValuePair
    ;

  # persistent + shared worlds
  shared = "PrismLauncher/worlds";

  # instances the shared worlds are linked into; if `tmpfs` is enabled, the
  # saves folder is mounted on tmpfs of `size`
  instances = [
    {
      name = "MCSR Ranked";
      tmpfs = true;
      size = "4G";
      worlds = [
        "mcsr-practise-v2.0.0"
      ];
    }
  ];

  savesOf = instance: "PrismLauncher/instances/${instance.name}/minecraft/saves";

  # links into the shared worlds for every instance
  worldLinks = listToAttrs (
    concatMap (
      instance:
      map (
        world:
        nameValuePair "${savesOf instance}/${world}" {
          source = "${directory}/${shared}/${world}";
        }
      ) instance.worlds
    ) instances
  );
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

  # mounts for instances with saves on tmpfs
  systemd.mounts =
    let
      owner = osConfig.users.users.${config.user};
      gid = osConfig.users.groups.${owner.group}.gid;
      uid = owner.uid;
    in
    map (instance: {
      what = "tmpfs";
      where = "${directory}/${savesOf instance}";
      type = "tmpfs";
      options = concatStringsSep "," [
        "size=${instance.size}"
        "mode=0755"
        "uid=${toString uid}"
        "gid=${toString gid}"
      ];
      # mounts must be created before hjem links shared worlds
      before = [ "hjem-activate@${config.user}.service" ];
      wantedBy = [ "hjem-activate@${config.user}.service" ];
    }) (filter (instance: instance.tmpfs) instances);

  xdg.data.files = worldLinks // {
    # ensure directory for shared worlds exists
    "${shared}".type = "directory";

    "PrismLauncher/prismlauncher.cfg" = {
      type = "copy";
      clobber = true;
      generator = generators.toINI { };
      value.General = {

        AutoCloseConsole = false;
        AutomaticJavaDownload = false;
        AutomaticJavaSwitch = true;
        CloseAfterLaunch = false;
        EnableFeralGamemode = true;
        EnableMangoHud = false;
        Env = builtins.toJSON (
          builtins.toJSON {
            LD_PRELOAD = "${getLib pkgs.jemalloc}/lib/libjemalloc.so.2";
          }
        );
        IgnoreJavaCompatibility = true;
        JavaPath = getExe pkgs.temurin-bin-21;
        JvmArgs = "-XX:+UseZGC -XX:+AlwaysPreTouch -Dgraal.TuneInlinerExploration=1 -XX:NmethodSweepActivity=1";
        MaxMemAlloc = 8192;
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

    "PrismLauncher/themes/tinted/theme.json" = {
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
  };

  # bind-mounted rather than symlinked: the tmpfs saves mounts above live
  # inside this directory
  preservation.preserveAt.state-dir.directories = [
    {
      directory = ".local/share/PrismLauncher";
      how = "bindmount";
    }
  ];
}
