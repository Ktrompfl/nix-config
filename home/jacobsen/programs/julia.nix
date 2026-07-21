{
  config,
  lib,
  pkgs,
  ...
}:
let
  lspProjectToml = pkgs.writeText "julia-languageserver-Project.toml" ''
    [deps]
    LanguageServer = "2b0e0bc5-e4fd-59b4-8912-456d1b03d8d7"
  '';
in
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

  # Bootstrap: copy Project.toml as a real file (not a nix store symlink) so
  # Pkg.instantiate() can write Manifest.toml alongside it, then instantiate.
  home.activation.juliaLspInstall = lib.hm.dag.entryAfter [ "writeBoundary" ] ''
    JULIA_ENV="${config.home.homeDirectory}/.julia/environments/languageserver"
    $DRY_RUN_CMD mkdir -p "$JULIA_ENV"
    $DRY_RUN_CMD cp --remove-destination ${lspProjectToml} "$JULIA_ENV/Project.toml"
    if [ ! -f "$JULIA_ENV/Manifest.toml" ]; then
      $DRY_RUN_CMD ${lib.getExe pkgs.julia-bin} \
        --project="$JULIA_ENV" \
        -e 'using Pkg; Pkg.instantiate()' \
        || echo "Julia LSP bootstrap failed — run manually"
    fi
  '';

  # make julia apps available
  home.sessionPath = [ "$HOME/.julia/bin" ];
}
