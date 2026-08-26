{
  config,
  lib,
  pkgs,
  ...
}:
let
  # Graphical programs started from a terminal would otherwise inherit the
  # terminal's slice; these wrappers hand them to the app slice instead.
  #
  # `--dir` because the unit starts in the home directory otherwise, and a
  # relative argument would quietly mean something else: `zed .` would open
  # `~` rather than wherever it was typed.
  runInAppSlice = {
    chromium = "chromium";
    code = "code";
    firefox = "firefox";
    libreoffice = "libreoffice";
    steam = "steam";
    vesktop = "vesktop";
    viewnior = "viewnior";
    vlc = "vlc";
    zathura = "zathura";
    zed = "zeditor";
  };

  aliases = {
    path = "echo $PATH";
    reboot = "systemctl reboot";

    # replace ls with eza
    ls = "eza -F --color=always";
    la = "eza -F -a --color=always";
    ll = "eza -F -l -a -g -h --color=always";
    lt = "eza -F -aT --color=always";
    l = "eza -F -a | grep -e '^.'"; # show only dotfiles

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
  };

  # fish resolves named colours through the terminal, which already carries the
  # scheme, so no hex is needed here.
  colors = {
    fish_color_normal = "normal";
    fish_color_command = "blue";
    fish_color_keyword = "magenta";
    fish_color_quote = "green";
    fish_color_redirection = "cyan";
    fish_color_end = "yellow";
    fish_color_error = "red";
    fish_color_param = "normal";
    fish_color_comment = "brblack";
    fish_color_selection = "--background=brblack";
    fish_color_search_match = "--background=brblack";
    fish_color_operator = "cyan";
    fish_color_escape = "cyan";
    fish_color_autosuggestion = "brblack";
    fish_color_cancel = "red";
    fish_color_option = "yellow";
    fish_color_valid_path = "--underline";

    fish_pager_color_progress = "brblack";
    fish_pager_color_prefix = "blue";
    fish_pager_color_completion = "normal";
    fish_pager_color_description = "brblack";
    fish_pager_color_selected_background = "--background=brblack";
  };

  section = lib.concatStringsSep "\n";
in
{
  packages = with pkgs; [
    fish
    starship
    direnv
    fzf
    nix-your-shell

    bat
    eza
    ripgrep
  ];

  xdg.config.files = {
    # base16 paints with terminal colours 0-15, so the scheme follows the
    # terminal rather than being restated here
    "bat/config".text = "--theme=base16\n";

    # conf.d is read before config.fish, in file name order
    "fish/conf.d/00-environment.fish".source = pkgs.writers.writeFish "00-environment.fish" (
      section (
        lib.mapAttrsToList (
          name: value:
          "set --global --export ${lib.escapeShellArg name} ${lib.escapeShellArg (toString value)}"
        ) config.environment.sessionVariables
      )
    );

    "fish/config.fish".source = pkgs.writers.writeFish "config.fish" (
      lib.concatStringsSep "\n\n" [
        "set fish_greeting # disable greeting"

        (section (lib.mapAttrsToList (name: value: "set -g ${name} ${value}") colors))

        (section (
          lib.mapAttrsToList (
            name: value: "alias -- ${lib.escapeShellArg name} ${lib.escapeShellArg value}"
          ) aliases
        ))

        (section (
          lib.mapAttrsToList (name: exe: ''
            function ${name} --wraps ${exe}
                ${lib.getExe pkgs.runapp} --dir=$PWD -- ${exe} $argv
            end'') runInAppSlice
        ))

        # Last, because each of these defines the prompt or a hook that has to
        # win over anything set above.
        (section [
          "${lib.getExe pkgs.fzf} --fish | source"
          "${lib.getExe pkgs.direnv} hook fish | source"
          "${lib.getExe pkgs.starship} init fish | source"
          "enable_transience"
          "${lib.getExe pkgs.nix-your-shell} fish | source"
        ])
      ]
    );

    # julia installs its apps here; PATH cannot come from environment.d
    "fish/conf.d/10-julia.fish".source =
      pkgs.writers.writeFish "10-julia.fish" "fish_add_path --append $HOME/.julia/bin";

    "direnv/lib/nix-direnv.sh".source = "${pkgs.nix-direnv}/share/nix-direnv/direnvrc";
  };

  preservation.preserveAt.state-dir.directories = [
    ".local/share/direnv"
    ".local/share/fish"
  ];
}
