{ pkgs, ... }:
let
  python-packages =
    ps: with ps; [
      # cli tools
      beautifulsoup4
      click
      gitpython

      # file formats
      pyyaml

      # scientific computing
      ipykernel
      networkx
      numpy
      scipy
      sympy
      pandas
      gurobipy
      jupyter
      huggingface-hub

      # matplotlib, enable additional backends and load respective platform packages
      (matplotlib.override {
        enableGtk3 = true;
        enableQt = true;
      })

      # gtk
      pygobject3
      pycairo
      gst-python

      # qt5
      pyqt5
    ];
  python = pkgs.python313.withPackages python-packages;
in
{
  packages = [
    python
    pkgs.gurobi
  ];

  # fallback jupyter kernel for zed repl
  xdg.data.files."jupyter/kernels/python3-nixpkgs/kernel.json" = {
    generator = (pkgs.formats.json { }).generate "python-kernel.json";
    value = {
      argv = [
        "${python}/bin/python"
        "-m"
        "ipykernel_launcher"
        "-f"
        "{connection_file}"
      ];
      display_name = "Python ${python.pythonVersion}";
      language = "python";
      metadata.debugger = true;
    };
  };

  environment.sessionVariables.GRB_LICENSE_FILE = "/home/jacobsen/.gurobi/gurobi.lic";

  preservation.preserveAt.state-dir = {
    files = [ ".python_history" ];
    directories = [ ".gurobi" ]; # directory for gurobi license file per host
  };
}
