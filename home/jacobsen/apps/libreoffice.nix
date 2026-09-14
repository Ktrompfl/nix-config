{ pkgs, ... }:
{
  apps.libreoffice = {
    package = pkgs.libreoffice;

    binaries = [
      "libreoffice"
      "sbase"
      "scalc"
      "sdraw"
      "simpress"
      "smath"
    ];

    appId = "org.libreoffice.LibreOffice";

    access = {
      documents = "rw";
      download = "rw";
    };

    jail.permissions = c: with c; [ viewer ];
  };
}
