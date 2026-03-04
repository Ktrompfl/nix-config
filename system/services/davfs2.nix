{ config, lib, ... }:
let
  cfg = config.services.davfs2;
in
lib.mkIf cfg.enable {
  # Don't lock files on the server when they are opened for writing to fix some issues with Nextcloud.
  # services.davfs2.settings.globalSection.use_locks = lib.mkDefault false;

  # Users must be member of this group in order to mount a davfs2 file system.
  users.users.jacobsen.extraGroups = [ "davfs2" ];

  sops.secrets."webdav/davfs2" = {
    mode = "0600";
    path = "/etc/davfs2/secrets";
  };
}
