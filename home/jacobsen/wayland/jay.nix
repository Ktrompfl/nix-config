{
  config,
  inputs,
  pkgs,
  ...
}:
let
  # track the mode stack in a tmpfile and signal waybar on changes to update a
  # mode indicator until jay / waybar support this natively via ipc
  jay-mode =
    let
      signal = 1;
      socket = "$XDG_RUNTIME_DIR/waybar/custom/mode";
    in
    pkgs.writeShellScript "jay-mode" /* sh */ ''
      mkdir -p "$(dirname "${socket}")"

      if [[ $1 ]]; then
        echo "$1" >> "${socket}"
      else
        sed -i '$ d' "${socket}" 2>/dev/null
      fi

      pkill -RTMIN+${toString signal} waybar
    '';

  # Tools the jay-config crate spawns or matches clients by name against.
  # jay-config itself only ever sees these by name (see ./jay-config/src),
  # so nothing else forces them into the user's environment.
  runtimeTools = with pkgs; [
    swaylock
    playerctl
    app2unit
    wl-mirror
    swaynotificationcenter
    waybar
    grim
    wayland-pipewire-idle-inhibit
    wl-clipboard
    wl-clip-persist
  ];
in
{
  wayland.windowManager.jay = {
    enable = true;

    # Configured via the jay-config Rust crate (compiled to config.so) rather
    # than the generated TOML file. See ./jay-config for the actual
    # configuration logic; this only feeds it the values that have to come
    # from the Nix world (store paths, stylix colors, home-manager settings).
    libraryConfig = pkgs.callPackage ./jay-config {
      inherit inputs;

      extraEnv = {
        JAY_MODE_SCRIPT = "${jay-mode}";

        CURSOR_THEME = config.home.pointerCursor.name;
        CURSOR_SIZE = config.home.pointerCursor.size;
        GTK_THEME = config.gtk.theme.name;
        QT_PLATFORM_THEME = config.qt.platformTheme.name;
        QT_STYLE = config.qt.style.name;
        MONOSPACE_FONT = config.stylix.fonts.monospace.name;

        BASE00 = config.lib.stylix.colors.base00;
        BASE01 = config.lib.stylix.colors.base01;
        BASE02 = config.lib.stylix.colors.base02;
        BASE03 = config.lib.stylix.colors.base03;
        BASE04 = config.lib.stylix.colors.base04;
        BASE05 = config.lib.stylix.colors.base05;
        BASE06 = config.lib.stylix.colors.base06;
        BASE07 = config.lib.stylix.colors.base07;
        BASE08 = config.lib.stylix.colors.base08;
        BASE09 = config.lib.stylix.colors.base09;
        BASE0A = config.lib.stylix.colors.base0A;
        BASE0B = config.lib.stylix.colors.base0B;
        BASE0C = config.lib.stylix.colors.base0C;
        BASE0D = config.lib.stylix.colors.base0D;
        BASE0E = config.lib.stylix.colors.base0E;
        BASE0F = config.lib.stylix.colors.base0F;
      };
    };
  };

  home.packages = runtimeTools;

  # persist logs and session management
  preservation.preserveAt.state-dir.directories = [ ".local/share/jay" ];
}
