{ pkgs, ... }:
{
  # fish is configured with home-manager, but enabling in system settings is still required for completions
  programs.fish = {
    enable = true;
    interactiveShellInit = ''
      set fish_greeting # disable greeting
    '';
  };

  # Using fish as the the login shell can cause compatibility issues. For example, certain recovery environments such as systemd's emergency mode to be completely broken when fish was set as the login shell.
  # Note: Lix forks before spawning the shell process so the parent process ends up being nix-shell and not fish.
  programs.bash = {
    enable = true;
    interactiveShellInit = ''
      # "check if parent process is not fish" && "make nested shells work properly"
      if grep -qv 'fish\|nix-shell' /proc/$PPID/comm && [[ $SHLVL == [12] ]]; then
          # set $SHELL for better integration with programs like nix shell, tmux, etc.
          SHELL=${pkgs.fish}/bin/fish exec ${pkgs.fish}/bin/fish
      fi
    '';
  };
}
