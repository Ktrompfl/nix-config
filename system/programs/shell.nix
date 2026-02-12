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
  # Here is one solution, which keeps bash as login shell and launches fish unless the parent process is already fish:
  programs.bash = {
    enable = true;
    interactiveShellInit = ''
      if [[ $(${pkgs.procps}/bin/ps --no-header --pid=$PPID --format=comm) != "fish" && -z ''${BASH_EXECUTION_STRING} ]]
      then
        shopt -q login_shell && LOGIN_OPTION='--login' || LOGIN_OPTION=""
        exec ${pkgs.fish}/bin/fish $LOGIN_OPTION
      fi
    '';
  };
}
