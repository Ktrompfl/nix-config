{ lib, ... }:
let
  # Matched by executable basename rather than by full (Nix store) path, so
  # this keeps working across rebuilds without needing store paths in the
  # config. Nix wraps some packages (e.g. via `wrapProgram`): the wrapper
  # script `exec`s into the real binary renamed to `.<name>-wrapped`, and
  # that's what ends up as the client's exe, not the plain `<name>`.
  clientCapabilities = {
    grim = [ "screencopy" ];
    swayidle = [ "idle-notifier" ];
    swaylock = [
      "layer-shell"
      "session-lock"
    ];
    swaync = [ "layer-shell" ];
    wayland-pipewire-idle-inhibit = [ "layer-shell" ];
    "wl-copy|wl-paste" = [ "data-control" ];
    wl-clip-persist = [ "data-control" ];
    wl-mirror = [ "screencopy" ];
  };
in
{
  # The wayland protocols each client is allowed to use.
  clients = lib.mapAttrsToList (pattern: capabilities: {
    match.exe-regex = "/\\.?(${pattern})(-wrapped)?$";
    inherit capabilities;
  }) clientCapabilities;
}
