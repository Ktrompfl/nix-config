{ pkgs, ... }:
{
  programs.ninjabrain-bot = {
    enable = true;
    # Runs the bot in an X server of its own, so that it works under Wayland.
    package = pkgs.ninjabrain-bot-xwayland;

    settings = {
      view = "detailed";
      size = "large";

      color_negative_coords = true;
      direction_help_enabled = true;
      mismeasure_warning_enabled = true;
      show_angle_errors = true;
      show_angle_updates = true;
      stronghold_display_type = "chunk";

      # measuring (boat eye)
      mc_version = "pre_119";
      use_precise_angle = true;
      angle_adjustment_type = "tall";
      angle_adjustment_display_type = "increments";
      default_boat_type = "green";
      sensitivity = 0.02291165;
      sigma_boat = 0.001;
      boat_error = 0.03;
    };
  };
}
