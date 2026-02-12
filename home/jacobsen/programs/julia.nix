{ pkgs, ... }:
{
  home.packages = with pkgs; [
    # nix-ld is installed so binary artifacts must not be patched on installation
    julia-bin
  ];

  preservation.preserveAt.state-dir.directories = [ ".julia" ];

  # automatically load revise and terminal loggers on startup
  xdg.configFile.".julia/config/startup.jl".text = /* julia */ ''
    atreplinit() do repl
        try
            using Logging: global_logger
            using TerminalLoggers: TerminalLogger
            global_logger(TerminalLogger())
        catch e
            @warn "Error initializing TerminalLoggers" exception=(e, catch_backtrace())
        end
        try
            using Revise
        catch e
            @warn "Error initializing Revise" exception=(e, catch_backtrace())
        end
    end
  '';
}
