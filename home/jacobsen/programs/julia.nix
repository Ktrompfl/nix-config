{ pkgs, ... }:
{
  packages = with pkgs; [
    # note: nix-ld is installed, so binary artifacts must not be patched on
    # installation; withPackages uses julia-bin underneath for that reason.
    (julia.withPackages [
      "LanguageServer"
      "Revise"
    ])

    runic
  ];

  preservation.preserveAt.state-dir.directories = [ ".julia" ];

  files.".julia/config/startup.jl".text = /* julia */ ''
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
}
