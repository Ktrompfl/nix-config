{
  config,
  lib,
  pkgs,
  ...
}:
{
  packages = [ pkgs.btop ];

  xdg.config.files = {
    "btop/btop.conf".text = ''
      color_theme = "tinted"
    '';

    "btop/themes/tinted.theme" = {
      generator = lib.generators.toKeyValueLines {
        mkKey = key: "theme[${key}]";
        quote = true;
      };

      value = with config.theme.colors.withHashtag; {
        main_bg = base00;
        main_fg = base05;
        title = base05;
        hi_fg = base0D;
        selected_bg = base03;
        selected_fg = base0D;
        inactive_fg = base04;
        graph_text = base05;
        meter_bg = base03;
        proc_misc = base05;
        cpu_box = base09;
        mem_box = base0B;
        net_box = base0C;
        proc_box = base0D;
        div_line = base01;
        temp_start = base0B;
        temp_mid = base0A;
        temp_end = base08;
        cpu_start = base0B;
        cpu_mid = base0A;
        cpu_end = base08;
        free_start = base0A;
        free_mid = base0B;
        free_end = base0B;
        cached_start = base0C;
        cached_mid = base0D;
        cached_end = base0D;
        available_start = base09;
        available_mid = base0F;
        available_end = base0F;
        used_start = base08;
        used_mid = base0E;
        used_end = base0E;
        download_start = base09;
        download_mid = base0F;
        download_end = base0F;
        upload_start = base08;
        upload_mid = base0E;
        upload_end = base0E;
        process_start = base0B;
        process_mid = base0A;
        process_end = base08;
      };
    };
  };
}
