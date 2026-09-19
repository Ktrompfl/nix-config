{
  jayLib,
  lib,
  pkgs,
  ...
}:
let
  inherit (jayLib)
    bar
    configureIdle
    defineAction
    exec
    idle
    ;
in
{
  # The actions that more than one shortcut - or the compositor itself -
  # refers to by name.
  actions = {
    lock = exec [
      (lib.getExe pkgs.swaylock)
      "--daemonize"
    ];

    suspend = exec [
      "systemctl"
      "suspend-then-hibernate"
    ];

    # The toml config has no state of its own, so the inhibitor toggle is
    # built out of two actions that redefine which one the shortcut runs
    # next. Anything else that changes the idle timeout (e.g. `jay idle`)
    # desynchronizes this.
    inhibit-idle = [
      (configureIdle { minutes = 0; }) # disables the timeout entirely
      (bar.idleInhibitor "on")
      (defineAction "toggle-idle-inhibitor" "$uninhibit-idle")
    ];
    uninhibit-idle = [
      (configureIdle idle)
      (bar.idleInhibitor "off")
      (defineAction "toggle-idle-inhibitor" "$inhibit-idle")
    ];
    toggle-idle-inhibitor = "$inhibit-idle";
  };
}
