{
  config,
  lib,
  pkgs,
  ...
}:
{
  home.packages = with pkgs; [
    # nix-ld is installed so binary artifacts must not be patched on installation
    julia-bin
  ];

  preservation.preserveAt.state-dir.directories = [ ".julia" ];

  # automatically load revise and terminal loggers on startup
  home.file.".julia/config/startup.jl".text = /* julia */ ''
    try
        using Revise
    catch e
        @warn "Error initializing Revise" exception=(e, catch_backtrace())
    end
    try
        using Logging: global_logger
        using TerminalLoggers: TerminalLogger
        global_logger(TerminalLogger())
    catch e
        @warn "Error initializing TerminalLoggers" exception=(e, catch_backtrace())
    end
  '';

  home.file.".julia/environments/languageserver/Project.toml".text = ''
    [deps]
    LanguageServer = "2b0e0bc5-e4fd-59b4-8912-456d1b03d8d7"
  '';

  # Bootstrap: run `julia --project=... -e 'using Pkg; Pkg.instantiate()'`
  # only when the Manifest doesn't exist yet
  home.activation.juliaLspInstall = lib.hm.dag.entryAfter [ "writeBoundary" ] ''
    if [ ! -f "${config.home.homeDirectory}/.julia/environments/languageserver/Manifest.toml" ]; then
      $DRY_RUN_CMD ${lib.getExe pkgs.julia-bin} \
        --project=@languageserver \
        -e 'using Pkg; Pkg.instantiate()' \
        || echo "Julia LSP bootstrap failed — run manually"
    fi
  '';
}
