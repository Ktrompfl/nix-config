{
  home.sessionVariables.TERMINAL = "footclient";

  programs.foot = {
    enable = true;
    server.enable = true;
    settings = {
      main = {
        shell = "fish";
        term = "xterm-256color";
      };
      bell.system = "no";
      cursor.style = "beam";
      cursor.blink = "true";
      mouse.hide-when-typing = "yes";
    };
  };
}
