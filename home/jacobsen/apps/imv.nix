{
  config,
  generators,
  pkgs,
  ...
}:
{
  apps.imv = {
    package = pkgs.imv;

    files.".config/imv/config" = {
      generator = generators.toINI { };
      value.options.background = config.theme.colors.withoutHashtag.base00;
      mutable = false;
    };

    jail.permissions = c: with c; [ viewer ];
  };
}
