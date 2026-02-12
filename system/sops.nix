{ inputs, pkgs, ... }:
{
  imports = [ inputs.sops-nix.nixosModules.sops ];

  environment.systemPackages = with pkgs; [
    age
    sops
  ];

  # To add a new host create a private age key with `age-keygen -o ~/.config/sops/age/keys.txt`
  # and add the public key together with the host name in `.sops.nix`.
  # Finally, update the encrypted secrets secrets with `sops updatekeys secrets/default.yaml`.

  # To edit secrets run `sops secrets/default.yaml`

  sops = {
    defaultSopsFile = ../secrets/default.yaml;
    age = {
      # the key file must be stored on a filesystem loaded early enough during boot, i.e. with
      # fileSystems."/persist".neededForBoot = true;
      keyFile = "/persist/sops/age/keys.txt";
      generateKey = true; # generate key if it does not exist
    };
  };
}
