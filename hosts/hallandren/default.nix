# if you want to use the key for interactive login be sure there is no trailing newline
# for example use `echo -n "password" > /tmp/secret.key`
# nix run github:nix-community/nixos-anywhere -- --disk-encryption-keys /tmp/secret.key /tmp/secret.key --flake /persist/nixos#hallandren --target-host nixos@192.168.178.21
{ pkgs, ... }:
{
  imports = [
    ./hardware-configuration.nix
    ./filesystem
    ./network
  ];

  host = {
    network.hostname = "hallandren";
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

  console.keyMap = "us";

  hardware = {
    amdgpu.initrd.enable = true;
    bluetooth.enable = true;
  };

  programs = {
    # applications
    corectrl.enable = true;
    piper.enable = true;
    steam.enable = true;
    via.enable = true;
    waywall.enable = true;

    # wayland session
    jay.enable = true;
    sway.enable = true;
    uwsm.enable = true;
  };

  services = {
    # regularly trim ssd devices
    fstrim.enable = true;

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
