{
  config,
  helpers,
  lib,
  osConfig,
  pkgs,
  ...
}:
let
  inherit (config.theme) fonts;
  inherit (lib)
    concatMap
    concatStringsSep
    filter
    getExe
    getLib
    listToAttrs
    nameValuePair
    ;

  data = ".local/share";

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

  savesOf = instance: "${data}/PrismLauncher/instances/${instance.name}/minecraft/saves";

  # links into the shared worlds for every instance, remade on each launch on
  # top of whatever the tmpfs mounts below left behind
  worldLinks = listToAttrs (
    concatMap (
      instance:
      map (
        world: nameValuePair "${savesOf instance}/${world}" "${data}/${shared}/${world}"
      ) instance.worlds
    ) instances
  );
in
{
  apps.prismlauncher = {
    package = pkgs.prismlauncher.override {
      additionalLibs = with pkgs; [
        libxtst
        libxkbcommon
        libxt
        libxinerama
      ];
      jdks = with pkgs; [
        temurin-bin-21
      ];
    };

    appId = "org.prismlauncher.PrismLauncher";

    jail.permissions =
      c: with c; [
        desktop
        network
      ];

    # the worlds the links below point at are shared between instances, so the
    # launcher never creates this itself
    directories = [ "${data}/${shared}" ];

    links = worldLinks;

    files = {
      "${data}/PrismLauncher/prismlauncher.cfg" = {
        generator = helpers.generators.toINI { };
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

      "${data}/PrismLauncher/themes/tinted/theme.json" = {
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
  };

  # The saves tmpfs is mounted on the host, inside the private home; bwrap
  # binds that home recursively, so the mount comes along. It has to be up
  # before the launcher starts, which is why it is wanted by the boot target
  # rather than ordered against hjem as it was when the worlds were linked at
  # activation time.
  systemd.mounts =
    let
      owner = osConfig.users.users.${config.user};
      gid = osConfig.users.groups.${owner.group}.gid;
      uid = owner.uid;
    in
    map (instance: {
      what = "tmpfs";
      where = "${config.apps.prismlauncher.storage}/${savesOf instance}";
      type = "tmpfs";
      options = concatStringsSep "," [
        "size=${instance.size}"
        "mode=0755"
        "uid=${toString uid}"
        "gid=${toString gid}"
      ];
      wantedBy = [ "multi-user.target" ];
    }) (filter (instance: instance.tmpfs) instances);
}
