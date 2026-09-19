{
  config,
  lib,
  pkgs,
  ...
}:
let
  # The sixteen base16 slots of the active theme scheme, lower-cased because
  # that is how the shared library configuration reads them back.
  base16 = lib.mapAttrs' (name: lib.nameValuePair (lib.toLower name)) (
    lib.filterAttrs (
      name: _: builtins.match "base0[0-9A-F]" name != null
    ) config.theme.colors.withoutHashtag
  );
in
{
  # The settings are split into the same parts as the shared library
  # configuration in ../../../../pkgs/jay-config-lib/src, so that the two can
  # be read side by side: bar.nix corresponds to its bar.rs, and so on. Each
  # part writes its own keys of `jay/config.toml` below; the module system
  # merges them, and two parts setting the same one disagree loudly.
  imports = [
    ./actions.nix
    ./bar.nix
    ./behavior.nix
    ./clients.nix
    ./inputs.nix
    ./outputs.nix
    ./shortcuts.nix
    ./theme.nix
    ./windows.nix
  ];

  packages = with pkgs; [
    jay

    runapp

    # The programs the two configurations share. The toml side refers to them
    # by store path, the shared library side needs them on `PATH`.
    jay-bar
    jay-screenshot

    # extra programs used in the jay config
    playerctl
    wl-mirror
  ];

  xdg.config.files = {
    # config.so takes precendence over config.toml
    "jay/config.so".source = "${pkgs.jay-config-lib}/lib/config.so";

    "jay/config.toml".generator = (pkgs.formats.toml { }).generate "jay-config.toml";

    "jay/theme.toml".text = lib.concatStrings (
      lib.mapAttrsToList (name: value: "${name} = ${builtins.toJSON (toString value)}\n") (
        base16
        // {
          monospace_font = config.theme.fonts.monospace.name;
        }
      )
    );
  };

  # persist logs and session management
  preservation.preserveAt.state-dir.directories = [ ".local/share/jay" ];
}
