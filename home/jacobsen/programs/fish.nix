{
  home.shell.enableFishIntegration = true;

  # man page cache generation slows build with fish enabled
  programs.man.generateCaches = false;

  programs.fish = {
    enable = true;
    interactiveShellInit = /* fish */ ''
      set fish_greeting # disable greeting
    '';
    shellAliases = {
      path = "echo $PATH";
      reboot = "systemctl reboot";

      # replace ls with exa
      ls = "eza -F --color=always";
      la = "eza -F -a --color=always";
      ll = "eza -F -l -a -g -h --color=always";
      lt = "eza -F -aT --color=always";
      l = "eza -F -a | grep -e '^\.'"; # show only dotfiles

      ".." = "cd ..";
      "..." = "cd ../..";
      "...." = "cd ../../..";
      "....." = "cd ../../../..";
      "......" = "cd ../../../../..";

      # add colors
      dir = "dir --color=auto";
      vdir = "vdir --color=auto";
      grep = "grep --color=auto";
      fgrep = "fgrep --color=auto";
      egrep = "egrep --color=auto";
      ip = "ip --color=auto";

      # human-readable formats
      df = "df -h";
      du = "du - h";
      free = "free -m";
      lsblk = "lsblk -o SIZE,NAME,VENDOR,MODEL,LABEL,FSTYPE,RO,TYPE,MOUNTPOINT,UUID";

      # verbose output
      cp = "cp -v";
      mv = "mv -v";

      # create parent directories
      md = "mkdir -p";

      # common options
      tarnow = "tar -acf";
      untar = "tar -zxvf";
      wget = "wget -c";

      zathura = "zathura --fork";
    };
  };
}
