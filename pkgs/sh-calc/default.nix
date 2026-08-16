{
  lib,
  python3Packages,
  wl-clipboard,
}:

python3Packages.buildPythonApplication {
  pname = "sh-calc";
  version = "0.1.0";
  pyproject = true;

  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./pyproject.toml
      ./LICENSE
      ./sh_calc
      ./tests
    ];
  };

  build-system = [ python3Packages.setuptools ];
  dependencies = [ python3Packages.numpy ];

  # wl-paste must be on PATH at runtime: the daemon spawns it to subscribe to
  # the wlr/ext data-control protocol.
  makeWrapperArgs = [
    "--prefix"
    "PATH"
    ":"
    (lib.makeBinPath [ wl-clipboard ])
  ];

  nativeCheckInputs = [ python3Packages.unittestCheckHook ];
  pythonImportsCheck = [ "sh_calc" ];

  # GPL-3 asks that the licence travel with the program.
  postInstall = ''
    install -Dm644 LICENSE -t $out/share/doc/sh-calc
  '';

  meta = {
    description = "Minecraft stronghold calculator for 1.16 boat eye, for Wayland";
    longDescription = ''
      Reads F3+C strings from the Wayland clipboard via the data-control protocol
      and takes commands over a unix socket, so hotkeys are ordinary compositor
      keybinds rather than global key grabs.  Publishes its report to a status
      file for an overlay to draw.

      Covers only the 1.16 boat-eye path, and only with the yaw grid anchored at
      0 -- Ninjabrain-Bot's green boat.  Nothing is measured and no boat is
      needed, but a run that anchors the grid elsewhere will be told the wrong
      stronghold with high confidence.

      Derivative work of Ninjabrain Bot, copyright (C) 2021 Filip Ryblad and
      contributors, GPL-3.0-only; modified 2026-08.  Not affiliated with or
      endorsed by it.  Full attribution in the sh_calc package docstring.
    '';
    license = lib.licenses.gpl3Only;
    platforms = lib.platforms.linux;
    mainProgram = "sh-calc";
  };
}
