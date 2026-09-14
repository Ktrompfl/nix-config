{
  config,
  lib,
  pkgs,
  ...
}:
{
  apps.imv = {
    package = pkgs.imv;

    files.".config/imv/config" = {
      generator = lib.generators.toINI { };
      value.options.background = config.theme.colors.withoutHashtag.base00;
      mutable = false;
    };

    jail.permissions = c: with c; [ viewer ];
  };
}
