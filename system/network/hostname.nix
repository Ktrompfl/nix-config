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

      # Every jail's uts namespace is named `jail`, and jails bind this file,
      # so this entry is what lets a sandbox resolve its own hostname.
      # Without this the lookup reaches resolved, which answers nxdomain only once llmnr and mdns time out, significantly slowing sandbox startup.
      hosts = {
        "127.0.0.1" = [ "jail" ];
        "::1" = [ "jail" ];
      };
    };
  };
}
