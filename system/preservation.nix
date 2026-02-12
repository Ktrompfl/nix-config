{ inputs, ... }:
{
  imports = [
    inputs.preservation.nixosModules.preservation
  ];

  preservation.preserveAt.state-dir = {
    directories = [
      "/etc/nix"
      {
        directory = "/var/lib/nixos";
        inInitrd = true;
      }
      "/var/lib/systemd"
    ];
    files = [ "/etc/adjtime" ];
  };
}
