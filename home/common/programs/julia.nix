{
  config,
  lib,
  pkgs,
  ...
}:
let
  julia = pkgs.julia-bin;
  depot = "${config.xdg.data.directory}/julia";

  # Packages are managed by Pkg, not nix (julia.withPackages re-runs Pkg in a
  # sandbox against a trimmed registry and breaks with every Pkg release).
  # Run `julia-install-tools` once on a new machine and again after a julia
  # upgrade (new minor versions get a fresh default environment); it is
  # idempotent, and `Pkg.update()` in each environment updates the tools.
  install-tools = pkgs.writeShellApplication {
    name = "julia-install-tools";
    runtimeInputs = [ julia ];
    text = ''
      julia --startup-file=no -e '
        using Pkg
        # default environment, stacked under every project: startup.jl
        # loads Revise, and the jupyter kernel below needs IJulia
        Pkg.activate()
        Pkg.add(["IJulia", "Revise"])
        # the julia-lsp language server runs in this shared environment
        Pkg.activate("languageserver"; shared = true)
        Pkg.add("LanguageServer")
        # formatter for jetls, installed as app under ${depot}/bin
        Pkg.Apps.add("Runic")
      '
    '';
  };
in
{
  packages = [
    julia
    install-tools
  ];

  # The trailing separator keeps julia's bundled depots, with the precompiled
  # standard library, behind this one.
  environment.sessionVariables = {
    JULIA_DEPOT_PATH = "${depot}:";
    JULIA_APPS_JULIA_CMD = lib.getExe julia;
  };

  # jupyter kernel for zed repl
  xdg.data.files."jupyter/kernels/julia-nixpkgs/kernel.json" = {
    generator = (pkgs.formats.json { }).generate "julia-kernel.json";
    value = {
      argv = [
        (lib.getExe julia)
        "-i"
        "--color=yes"
        "--project=@."
        "-e"
        "import IJulia; IJulia.run_kernel()"
        "{connection_file}"
      ];
      display_name = "Julia ${lib.versions.majorMinor julia.version}";
      language = "julia";
      env = { };
      interrupt_mode = "signal";
    };
  };

  xdg.data.files."julia/config/startup.jl".text = /* julia */ ''
    try
        using Revise
    catch e
        @warn "Error initializing Revise" exception=(e, catch_backtrace())
    end
  '';
}
