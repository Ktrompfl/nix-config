# Behaviour that both jay configurations need: the toml one in
# home/jacobsen/wayland/jay and the shared library one in ../jay-config-lib.
# Neither side reimplements what lives here, so both really do run the same
# code; the toml side calls these by store path, the shared library by name
# through `PATH`.
{
  cliphist,
  coreutils,
  fuzzel,
  grim,
  jay,
  jq,
  libnotify,
  satty,
  systemd,
  wl-clipboard,
  writeShellApplication,
}:
let
  # The object paths of the two `custom_dbus` blocks that show compositor
  # state. See ../../home/jacobsen/wayland/i3status-rust.nix for the
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

  jay-clipboard-history = writeShellApplication {
    name = "jay-clipboard-history";
    runtimeInputs = [
      cliphist
      fuzzel
      wl-clipboard
    ];
    text = ''
      cliphist list | fuzzel --dmenu --with-nth 2 | cliphist decode | wl-copy
    '';
  };

  jay-screenshot = writeShellApplication {
    name = "jay-screenshot";
    runtimeInputs = [
      grim
      jay
      jq
      libnotify
      satty
    ];
    text = ''
      mode=''${1:-window}

      fail() {
        notify-send --urgency=critical "screenshot" "$1"
        exit 1
      }

      # `jay --json` writes one JSON object per matched node; slurping keeps
      # the first one and yields nothing at all when there was no match.
      query() {
        jay --json tree query "$@" | jq -s '.[0] // empty'
      }

      geometry() {
        jq -er '"\(.position.x1),\(.position.y1) \(.position.width)x\(.position.height)"' <<<"$1"
      }

      # Everything but `region` derives its geometry from the focused window,
      # because that is the only node a caller without access to the tree can
      # name: the focused window knows its own position, and its workspace
      # knows both its position and the output it is on. A caller that can
      # compute the geometry itself passes `region` instead and is not
      # restricted to workspaces and outputs that have a window on them.
      if [[ $mode != region ]]; then
        window=$(query match-windows -e 'focused = true')
        [[ -n $window ]] || fail "no focused window"
      fi

      case $mode in
        region)
          [[ $# -eq 2 ]] || fail "usage: jay-screenshot region '<x>,<y> <w>x<h>'"
          args=(-g "$2")
          ;;
        window)
          args=(-g "$(geometry "$window")")
          ;;
        workspace | output)
          name=$(jq -er '.workspace' <<<"$window") || fail "focused window is not on a workspace"
          workspace=$(query workspace-name "$name")
          [[ -n $workspace ]] || fail "workspace $name not found"
          if [[ $mode == workspace ]]; then
            args=(-g "$(geometry "$workspace")")
          else
            args=(-o "$(jq -er '.output' <<<"$workspace")")
          fi
          ;;
        *)
          fail "unknown mode: $mode"
          ;;
      esac

      grim "''${args[@]}" - | satty --filename -
    '';
  };
}
