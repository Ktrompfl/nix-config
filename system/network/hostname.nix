{ config, lib, ... }:
{
  options.host.network.hostname = lib.mkOption {
    type = lib.types.str;
    default = "null";
    description = "Hostname of system";
  };

  config = {
    assertions = [
      {
        assertion = config.host.network.hostname != "null";
        message = "[host.network.hostname] Enter a hostname to add network uniqueness";
      }
    ];

    networking = {
      hostName = config.host.network.hostname;
      # generate host ID from hostname
      hostId = builtins.substring 0 8 (builtins.hashString "sha256" config.networking.hostName);

      # the jail's hostname; otherwise resolved only answers after llmnr and
      # mdns time out, which slows down every jail start
      hosts = {
        "127.0.0.1" = [ "jail" ];
        "::1" = [ "jail" ];
      };
    };
  };
}
