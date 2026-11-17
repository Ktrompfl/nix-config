{ config, lib, ... }:
let
  base16 = lib.mapAttrs' (name: lib.nameValuePair (lib.toLower name)) (
    lib.filterAttrs (
      name: _: builtins.match "base0[0-9A-F]" name != null
    ) config.theme.colors.withoutHashtag
  );

  theme = lib.concatStrings (
    lib.mapAttrsToList (name: value: "${name} = ${builtins.toJSON (toString value)}\n") (
      base16
      // {
        sans_font = "${config.theme.fonts.sansSerif.name} ${toString config.theme.fonts.sizes.desktop}";
      }
    )
  );
in
{
  xdg.config.files = {
    "jay/theme.toml".text = theme;
    "getaway/theme.toml".text = theme;
  };
}
