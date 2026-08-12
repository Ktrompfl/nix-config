{ lib, ... }:
let
  # find names of outputs with 'jay randr'
  #
  # `workspaces` is not part of an output's configuration; it is turned into
  # the `workspaces` table below. A workspace starts out on the first
  # connected output that lists it, which is how the two desks and the laptop
  # end up with the same workspaces regardless of what is plugged in.
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
      workspaces = [
        "1"
        "2"
        "3"
        "4"
        "5"
      ];
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
      workspaces = [ "0" ];
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
      workspaces = [
        "1"
        "2"
        "3"
        "4"
        "5"
      ];
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
      workspaces = [
        "6"
        "7"
        "8"
        "9"
        "0"
      ];
    }
  ];

  # Inverts the `workspaces` lists above: every workspace mentioned by any
  # output gets the outputs that mention it, in the order they are declared.
  workspaceNames = lib.unique (lib.concatMap (output: output.workspaces) outputs);
  initialOutput = name: {
    initial-output = map (output: { inherit (output) name; }) (
      lib.filter (output: lib.elem name output.workspaces) outputs
    );
  };
in
{
  # The displays and which of them a workspace starts out on.
  outputs = map (output: removeAttrs output [ "workspaces" ]) outputs;
  workspaces = lib.genAttrs workspaceNames initialOutput;
}
