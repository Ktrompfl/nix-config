{ lib, pkgs, ... }:
{
  packages = with pkgs; [
    pwvucontrol
    alsa-scarlett-gui # mixer for the focusrite interface
  ];

  # Plays the FOX mic on the Scarlett's first output pair. Started on demand
  # rather than with the session, since its small quantum drags the whole
  # graph down to it for as long as it runs. The two devices have separate
  # clocks, so pipewire resamples between them.
  wayland.services.mic-monitor = {
    description = "Monitor the USB mic on the audio interface";
    wantedBy = [ ];
    slice = "session";
    serviceConfig = {
      ExecStart = lib.concatStringsSep " " [
        "${pkgs.pipewire}/bin/pw-loopback"
        "--name=mic-monitor"
        "--capture=alsa_input.usb-Beyerdynamic_FOX_5.00-00.mono-fallback"
        "--playback=alsa_output.usb-Focusrite_Scarlett_4i4_USB_D87VB4U990ABDF-00.pro-output-0"
        ''--capture-props="node.latency=64/48000 node.dont-fallback=true"''
        # the pro-audio profile names its channels AUX0 to AUX3
        ''--playback-props="node.latency=64/48000 node.dont-fallback=true audio.position=[AUX0 AUX1]"''
      ];
    };
  };
}
