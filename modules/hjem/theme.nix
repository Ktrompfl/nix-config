{ lib, osConfig, ... }:
{
  options.theme = lib.mkOption {
    type = lib.types.raw;
    readOnly = true;
    description = "The system theme, mirrored from NixOS ../../system/theme.";
  };

  config.theme = osConfig.theme;
}
