{
  config,
  lib,
  pkgs,
  ...
}:
let
  base16 = lib.mapAttrs' (name: lib.nameValuePair (lib.toLower name)) (
    lib.filterAttrs (
      name: _: builtins.match "base0[0-9A-F]" name != null
    ) config.theme.colors.withoutHashtag
  );
in
{
  packages = with pkgs; [
    runapp
    jay-screenshot
    playerctl
    wl-mirror
  ];

  systemd.services.jay = {
    description = "jay wayland compositor";
    # inherit the login PATH that jay-session imports, instead of the default
    # one of generated units
    environment.PATH = lib.mkForce null;
    serviceConfig = {
      # run with security wrapper to raise jay's scheduling priority
      ExecStart = "/run/wrappers/bin/jay run";
      Slice = "session.slice";
      # the config reports readiness once jay can render
      Type = "notify";
      NotifyAccess = "all";
      # without the config, jay keeps running with its defaults
      TimeoutStartSec = "infinity";
    };
  };

  # a drop-in: hjem ignores `overrideStrategy` and would replace the whole unit
  xdg.config.files."systemd/user/graphical-session.target.d/jay.conf".text = ''
    [Unit]
    BindsTo=jay.service
    After=jay.service
  '';

  xdg.config.files = {
    "jay/config.so".source = "${pkgs.jay-config-lib}/lib/config.so";

    "jay/theme.toml".text = lib.concatStrings (
      lib.mapAttrsToList (name: value: "${name} = ${builtins.toJSON (toString value)}\n") (
        base16
        // {
          monospace_font = config.theme.fonts.monospace.name;
        }
      )
    );
  };
}
