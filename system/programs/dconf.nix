{
  programs.dconf.profiles.user.databases = [
    {
      settings."org/gnome/desktop/interface" = {
        color-scheme = "prefer-dark";
        gtk-theme = "adw-gtk3";
      };

      # the portal has no reason to report anything else
      locks = [ "/org/gnome/desktop/interface/color-scheme" ];
    }
  ];
}
