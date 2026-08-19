{
  config,
  lib,
  pkgs,
  ...
}:
{
  packages = [ pkgs.mangohud ];

  xdg.config.files."MangoHud/MangoHud.conf" = {
    generator = lib.generators.toKeyValueLines { flags = true; };

    value = with config.theme.colors.withoutHashtag; {
      alpha = "1.000000";
      background_alpha = "1.000000";
      background_color = base00;
      text_color = base05;
      text_outline_color = base00;
      battery_color = base04;
      cpu_color = base0D;
      gpu_color = base0B;
      vram_color = base0C;
      engine_color = base0E;
      io_color = base0A;
      frametime_color = base0B;
      media_player_color = base05;
      wine_color = base0E;
      cpu_load_color = "${base0B}, ${base0A}, ${base08}";
      gpu_load_color = "${base0B}, ${base0A}, ${base08}";
      fps_color = "${base0B}, ${base0A}, ${base08}";
      font_size = config.theme.fonts.sizes.terminal;
      font_size_text = config.theme.fonts.sizes.terminal;
      font_scale = "1.333330";

      fps = true;
      show_fps_limit = true;
      cpu_stats = true;
      cpu_temp = true;
      cpu_mhz = true;
      cpu_power = true;
      gpu_stats = true;
      gpu_temp = true;
      gpu_mhz = true;
      gpu_power = true;
      ram = true;
      vram = true;
      hud_compact = true;
      gamemode = true;
    };
  };
}
