{ pkgs, ... }:
let
  power-menu = pkgs.writeShellApplication {
    name = "power-menu";
    runtimeInputs = with pkgs; [
      fuzzel
      swaylock
    ];
    text = ''
      if systemctl --user is-active -q idle.service; then
        idle="Inhibit idle"
      else
        idle="Allow idle"
      fi

      # label and icon from the icon theme, in menu order
      entries=(
        "Lock" "system-lock-screen"
        "Suspend" "system-suspend"
        "$idle" "caffeine"
        "Log out" "system-log-out"
        "Reboot" "system-reboot"
        "Reboot to UEFI" "firmware-manager"
        "Shut down" "system-shutdown"
      )

      # fuzzel prints nothing when the menu is dismissed
      index=$(
        printf '%s\0icon\x1f%s\n' "''${entries[@]}" |
          fuzzel --dmenu --index --lines=$((''${#entries[@]} / 2)) --prompt='$ '
      ) || exit 0

      case $index in
        0) swaylock --daemonize ;;
        1) systemctl suspend-then-hibernate ;;
        2)
          if [ "$idle" = "Inhibit idle" ]; then
            systemctl --user stop idle.service
          else
            systemctl --user start idle.service
          fi
          ;;
        3) loginctl terminate-session "''${XDG_SESSION_ID:?}" ;;
        4) systemctl reboot ;;
        5) systemctl reboot --firmware-setup ;;
        6) systemctl poweroff ;;
      esac
    '';
  };
in
{
  packages = [ power-menu ];
}
