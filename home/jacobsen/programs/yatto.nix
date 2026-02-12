{ config, pkgs, ... }:
{
  home.packages = with pkgs; [
    yatto
  ];

  home.file."${config.xdg.configHome}/yatto/config.toml".source =
    (pkgs.formats.toml { }).generate "yatto-config"
      {
        assignee = {
          show = false;
          show_printer = false;
        };

        author = {
          show = false;
          show_printer = false;
        };
        colors =
          let
            palette = config.lib.stylix.colors.withHashtag;
          in
          {
            form.theme = "Base16";
            badge_text_dark = palette.base00;
            badge_text_light = palette.base00;
            blue_dark = palette.base0B;
            blue_light = palette.base0B;
            green_dark = palette.base0C;
            green_light = palette.base0C;
            indigo_dark = palette.base0D;
            indigo_light = palette.base0D;
            orange_dark = palette.base09;
            orange_light = palette.base09;
            red_dark = palette.base08;
            red_light = palette.base08;
            vividred_dark = palette.base0A;
            vividred_light = palette.base0A;
            yellow_dark = palette.base09;
            yellow_light = palette.base09;
          };

        git = {
          default_branch = "main";
          remote = {
            enable = true;
            name = "origin";
            url = "git@github.com:ktrompfl/todo.git";
          };
        };

        jj = {
          default_branch = "main";

          remote = {
            colocate = false;
            enable = false;
            name = "origin";
            url = "git@github.com:ktrompfl/todo.git";
          };
        };

        vcs.backend = "git";
        storage.path = "/home/jacobsen/.local/share/yatto";
      };

  preservation.preserveAt.state-dir.directories = [ ".local/share/yatto" ];
}
