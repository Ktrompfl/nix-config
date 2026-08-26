{ config, pkgs, ... }:
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

  environment.sessionVariables = {
    GRB_LICENSE_FILE = "/home/jacobsen/.gurobi/gurobi.lic";

    # out of $HOME and into the already-preserved XDG state directory. readline
    # rewrites the history file through a temporary file and a rename, which
    # fails with EBUSY against a bind-mounted file and silently detaches a
    # symlinked one; inside a preserved directory the rename stays on /cache.
    PYTHON_HISTORY = "${config.xdg.state.directory}/python_history";
  };

  preservation.preserveAt.state-dir.directories = [
    ".gurobi" # directory for gurobi license file per host
  ];
}
