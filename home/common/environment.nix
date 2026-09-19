{ config, lib, ... }:
{
  # systemd reads environment.d on its own, so this is where session variables
  # have to land. It is not a shell: `$VAR` is only substituted from variables
  # already in the manager's environment, so a line like `PATH=$HOME/bin:$PATH`
  # silently truncates PATH rather than extending it. PATH is therefore refused
  # here and belongs in the shell, see ./programs/shell.nix.
  assertions = [
    {
      assertion = !(config.environment.sessionVariables ? PATH);
      message = "PATH cannot be set through environment.d; add it in the shell instead.";
    }
  ];

  xdg.config.files."environment.d/10-hjem.conf".text = lib.concatLines (
    lib.mapAttrsToList (name: value: "${name}=${value}") config.environment.sessionVariables
  );
}
