{
  config,
  lib,
  pkgs,
  ...
}:
{
  packages = [ pkgs.imv ];

  xdg.config.files."imv/config" = {
    generator = lib.generators.toINI { };

    value.options.background = config.theme.colors.withoutHashtag.base00;
  };
}
