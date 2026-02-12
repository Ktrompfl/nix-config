{ ... }:
{
  # overlay for displaying various information like fps and cpu/gpu temeprature
  programs.mangohud = {
    enable = true;
    enableSessionWide = false;
    settings = {
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
      # # Hide until toggled
      # no_display = true;

      # toggle_hud = "Shift_L+F1";
      # toggle_hud_position = "Shift_L+F2";
      # toggle_logging = "Shift_L+F3";
      # reload_cfg = "Shift_L+F4";
    };
  };
}
