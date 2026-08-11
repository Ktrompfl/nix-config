{ lib, ... }:
let
  # Workspaces initially live on the laptop panel or the desk's horizontal
  # monitor, whichever of the two is connected; the second half of them on
  # the desk's vertical monitor. `0` is the beamer's workspace whenever the
  # beamer is around.
  initialOutput =
    output: names:
    lib.genAttrs names (_: {
      initial-output = output;
    });
in
{
  # The displays and which of them a workspace starts out on.
  wayland.windowManager.jay.settings = {
    # find names of outputs with 'jay randr'
    outputs = [
      {
        name = "laptop-integrated";
        match.connector = "eDP-1";
        x = 0;
        y = 0;
        mode = {
          width = 1920;
          height = 1080;
          refresh-rate = 60.0;
        };
      }
      {
        # TODO: automatically spawn wl-mirror
        name = "beamer";
        match.model = "EPSON PJ";
        x = 0;
        y = 1080;
        mode = {
          width = 1920;
          height = 1080;
          refresh-rate = 60.0;
        };
      }
      {
        name = "horizontal";
        match.model = "VG270U P";
        x = 0;
        y = 240;
        scale = 1.0;
        mode = {
          width = 2560;
          height = 1440;
          refresh-rate = 143.995;
        };
        transform = "none";
        vrr.mode = "variant3"; # if this does not work try variant2
      }
      {
        name = "vertical";
        match.model = "BenQ GL2480";
        x = 2560;
        y = 0;
        scale = 1.0;
        mode = {
          width = 1920;
          height = 1080;
          refresh-rate = 60.0;
        };
        transform = "rotate-90";
      }
    ];

    workspaces =
      initialOutput
        [
          { name = "laptop-integrated"; }
          { name = "horizontal"; }
        ]
        [
          "1"
          "2"
          "3"
          "4"
          "5"
        ]
      // initialOutput { name = "vertical"; } [
        "6"
        "7"
        "8"
        "9"
      ]
      //
        initialOutput
          [
            { name = "beamer"; }
            { name = "vertical"; }
          ]
          [ "0" ];
  };
}
