{ lib, ... }:
{
  options.systemd.mounts = lib.mkOption {
    type = with lib.types; listOf attrs;
    default = [ ];
    description = ''
      Mount units for this user, in the shape of the NixOS `systemd.mounts`
      entries they are folded into by `home/default.nix`.

      The user manager has no mount units of its own, so these are realised by
      the system manager and are not namespaced to the user in any way. Order
      anything that has to see the mount after it explicitly.
    '';
  };
}
