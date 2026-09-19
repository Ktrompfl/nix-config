{ pkgs, ... }:
{
  apps.zotero = {
    package = pkgs.zotero;
    appId = "org.zotero.zotero";

    # the library itself lives in the private home; these are for attaching
    # papers from elsewhere and saving them back out
    access = {
      documents = "ro";
      download = "rw";
    };

    jail.permissions =
      c: with c; [
        desktop
        network
        notifications
      ];
  };
}
