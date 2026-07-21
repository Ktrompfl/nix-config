{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.services.pipewire;
in
lib.mkIf cfg.enable {
  # rtkit allows pipewire to use the realtime scheduler for increased performance
  security.rtkit.enable = lib.mkDefault true;

  services.pipewire = {
    alsa.enable = lib.mkDefault true;
    alsa.support32Bit = lib.mkDefault true;
    pulse.enable = lib.mkDefault true;
    wireplumber.enable = lib.mkDefault true;

    # noise suppression
    extraLadspaPackages = [ pkgs.rnnoise-plugin ];
    extraConfig.pipewire."99-input-denoising" = {
      "context.modules" = [
        {
          name = "libpipewire-module-filter-chain";
          args = {
            "node.description" = "Noise Canceling source";
            "media.name" = "Noise Canceling source";
            "filter.graph" = {
              nodes = [
                {
                  type = "ladspa";
                  name = "rnnoise";
                  plugin = "librnnoise_ladspa";
                  label = "noise_suppressor_stereo";
                  control = {
                    "VAD Threshold (%)" = 95.0;
                    "VAD Grace Period (ms)" = 200;
                    "Retroactive VAD Grace (ms)" = 0;
                  };
                }
              ];
            };
            "capture.props" = {
              "node.passive" = true;
              "audio.rate" = 48000;
            };
            "playback.props" = {
              "media.class" = "Audio/Source";
              "audio.rate" = 48000;
            };
          };
        }
      ];
    };
  };

  users.users.jacobsen.extraGroups = [
    "audio"
    "video"
  ];

  preservation.preserveAt.state-dir.directories = [ "/var/lib/pipewire" ];
}
