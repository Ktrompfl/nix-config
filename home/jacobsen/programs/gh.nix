{ pkgs, ... }:
{
  packages = [ pkgs.gh ];

  xdg.config.files."gh/config.yml" = {
    generator = (pkgs.formats.yaml { }).generate "gh-config.yml";
    value = {
      aliases = { };
      editor = "";
      git_protocol = "ssh";
      version = "1";
    };
  };
}
