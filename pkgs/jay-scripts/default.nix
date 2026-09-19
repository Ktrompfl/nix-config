# Behaviour that both jay configurations need: the toml one in
# home/graphical/wayland/jay and the shared library one in ../jay-config-lib.
{
  coreutils,
  systemd,
  writeShellApplication,
}:
let
  # The object paths of the two `custom_dbus` blocks that show compositor
  # state. See ../../home/graphical/wayland/status.nix for the
  # receiving end; it repeats the "off" icon as the format's fallback.
  modePath = "/mode";
  idleInhibitorPath = "/idle_inhibitor";

  idleInhibitorOnIcon = "󰅶";
  idleInhibitorOffIcon = "󰾪";
in
{
  # Everything shown in the bar is rendered by i3status-rust. The two segments
  # that reflect compositor state - the active input mode and the idle
  # inhibitor - cannot be observed from the outside, so the configurations push
  # them into `custom_dbus` blocks whenever the state changes.
  jay-bar = writeShellApplication {
    name = "jay-bar";
    runtimeInputs = [
      coreutils
      systemd
    ];
    text = ''
      usage() {
        echo "usage: jay-bar mode <name> | idle-inhibitor on|off | init" >&2
        exit 1
      }

      set_text() {
        busctl --user call rs.i3status "$1" rs.i3status.custom SetText ss "$2" "" >/dev/null
      }

      case "''${1-}" in
        mode)
          [[ $# -eq 2 ]] || usage
          set_text ${modePath} "''${2^^}"
          ;;
        idle-inhibitor)
          case "''${2-}" in
            on) set_text ${idleInhibitorPath} '${idleInhibitorOnIcon}' ;;
            off) set_text ${idleInhibitorPath} '${idleInhibitorOffIcon}' ;;
            *) usage ;;
          esac
          ;;
        # A `custom_dbus` block stays invisible until something has pushed a
        # value into it, so both of them are seeded once the bar is up.
        # i3status-rust only claims the bus name a moment after jay spawns it,
        # hence the retry.
        init)
          for _ in {1..100}; do
            if set_text ${modePath} NORMAL 2>/dev/null \
              && set_text ${idleInhibitorPath} '${idleInhibitorOffIcon}' 2>/dev/null; then
              exit 0
            fi
            sleep 0.1
          done
          exit 1
          ;;
        *) usage ;;
      esac
    '';
  };
}
