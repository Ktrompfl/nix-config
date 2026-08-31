{ pkgs, ... }:
let
  # note: nix-ld is installed, so binary artifacts must not be patched on
  # installation; withPackages uses julia-bin underneath for that reason
  julia = pkgs.julia.withPackages [
    "IJulia"
    "Revise"
  ];
in
{
  packages = [
    julia
    pkgs.runic
  ];

  preservation.preserveAt.state-dir.directories = [ ".julia" ];

  # jupyter kernel for zed repl
  xdg.data.files."jupyter/kernels/julia-nixpkgs/kernel.json" = {
    generator = (pkgs.formats.json { }).generate "julia-kernel.json";
    value = {
      argv = [
        "${julia}/bin/julia"
        "-i"
        "--color=yes"
        "--project=@."
        "-e"
        "import IJulia; IJulia.run_kernel()"
        "{connection_file}"
      ];
      display_name = "Julia ${pkgs.lib.versions.majorMinor pkgs.julia.version}";
      language = "julia";
      env = { };
      interrupt_mode = "signal";
    };
  };

  files.".julia/config/startup.jl".text = /* julia */ ''
    try
        using Revise
    catch e
        @warn "Error initializing Revise" exception=(e, catch_backtrace())
    end
  '';
}
