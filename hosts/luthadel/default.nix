{ lib, pkgs, ... }:
{
  imports = [
    ./hardware-configuration.nix
    ./filesystem
    ./network
  ];

  host = {
    network.hostname = "luthadel";
  };

  # This value determines the NixOS / Home Manager release from which the default
  # settings for stateful data, like file locations and database versions
  # on your system were taken. It‘s perfectly fine and recommended to leave
  # this value at the release version of the first install of this system.
  # Before changing this value read the documentation for this option
  # (e.g. man configuration.nix or on https://nixos.org/nixos/options.html).
  system.stateVersion = "24.05";
  home-manager.users.jacobsen.home.stateVersion = "24.05";

  boot = {
    loader = {
      systemd-boot.enable = true;
      efi.canTouchEfiVariables = true;
    };

    kernelParams = [
      "amd_pstate=active"
      "amd_prefcore=enable"
    ];

    plymouth.enable = true;
  };

  console.keyMap = "de";

  hardware.bluetooth.enable = true;

  programs = {
    # applications
    piper.enable = true;
    steam.enable = true;
    via.enable = true;

    # wayland session
    jay.enable = true;
    sway.enable = true;
    uwsm.enable = true;
  };

  services = {
    # regularly trim ssd devices
    fstrim.enable = true;

    # keep firmware up to date
    fwupd.enable = true;

    # power saving
    tlp.enable = true;
    upower.enable = true;

    # brightness controls
    actkbd = {
      enable = true;
      bindings = [
        {
          keys = [ 224 ];
          events = [ "key" ];
          command = "${lib.getExe pkgs.brightnessctl} set 4%-";
        }
        {
          keys = [ 225 ];
          events = [ "key" ];
          command = "${lib.getExe pkgs.brightnessctl} set 4%+";
        }
      ];
    };

    # wayland session
    pipewire.enable = true;

    # greeter
    greetd =
      let
        # bypass jay.desktop entry
        session = "uwsm start -F jay run";
      in
      {
        enable = true;
        settings = {
          terminal.vt = 1;

          # auto-login into session without requiring a password on initial boot (disk decryption prompts for a password anyways)
          initial_session = {
            command = session;
            user = "jacobsen";
          };

          # require username and password on subsequent logins
          default_session = {
            command = ''${pkgs.greetd}/bin/agreety --cmd "${session}"'';
            user = "greeter";
          };
        };
      };
  };
}
