{ lib, ... }:
{
  options.systemd.tmpfiles.rules = lib.mkOption {
    type = with lib.types; listOf str;
    default = [ ];
    example = [ "d %h/.cache/scratch 0700 - - 7d" ];
    description = ''
      systemd-tmpfiles rules for this user, folded into the NixOS
      `systemd.user.tmpfiles.users.<name>.rules` by `home/default.nix`.

      These run in the user manager, so `%h` is the user's home and the owner
      and group fields can be left as `-`. Ageing is applied by
      `systemd-tmpfiles-clean.timer`, which the user manager runs daily.
    '';
  };

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
