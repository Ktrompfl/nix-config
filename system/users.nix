{ config, ... }:
{
  sops.secrets."system/password".neededForUsers = true;

  users = {
    # don't allow mutation outside of this config, in particular the password is fixed to the initial password
    mutableUsers = false;

    users = {
      jacobsen = {
        isNormalUser = true;
        description = "Nicolaus Jacobsen";
        extraGroups = [
          "input"
          "wheel"
        ];
        # to generate a hashed password run: nix-shell --run 'mkpasswd -m SHA-512 -s' -p mkpasswd
        hashedPasswordFile = config.sops.secrets."system/password".path;
        openssh.authorizedKeys.keys = [
          "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIHoqUMNZEsWLEs+LZMH0uWmHkGjvVWny0KrX8OHhmRkD jacobsen@nixos" # hallandren
          "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIFJQ5fTgvbC7yucalBKGLOAFLgIbUBz4FpxlALnYYCh3 jacobsen@nixos" # luthadel
        ];
      };
      # disable root login
      root = {
        hashedPassword = "!";
        initialHashedPassword = "!";
      };
    };
  };
}
