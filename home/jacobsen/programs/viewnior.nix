{ pkgs, ... }:
{
  home.packages = [ pkgs.viewnior ];

  xdg.mimeApps.defaultApplications = {
    "image/gif" = "viewnior.desktop";
    "image/jpeg" = "viewnior.desktop";
    "image/png" = "viewnior.desktop";
  };
}
