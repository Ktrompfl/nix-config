{
  config,
  helpers,
  pkgs,
  ...
}:
{
  packages = [ pkgs.imv ];

  xdg.config.files."imv/config" = {
    generator = helpers.generators.toINI { };
    value.options.background = config.theme.colors.withoutHashtag.base00;
  };
}
