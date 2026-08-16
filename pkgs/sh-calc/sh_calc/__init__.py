"""Minimal stronghold calculator for 1.16 boat eye, for Wayland.

Watches the clipboard for F3+C strings via the wlr/ext data-control protocol
(`wl-paste --watch`) and takes commands over a unix socket, so hotkeys are just
compositor keybinds running `sh-calc <command>`.

    sh-calc daemon                # run this in a terminal
    sh-calc inc | dec             # bind these in your compositor
    sh-calc undo | redo | reset

Every throw is treated as a full-precision boat throw with the yaw grid anchored
at 0, which is Ninjabrain-Bot's green boat: no boat is measured and no angle is
assumed beyond that anchor.  If the grid is anchored somewhere else the answer
will be confidently wrong, so this is the one assumption worth knowing about.

Two modules: `stronghold` is where the stronghold is, `daemon` is the program
around it.

Derivative work of Ninjabrain Bot, copyright (C) 2021 Filip Ryblad and
contributors, GPL-3.0-only: https://github.com/Ninjabrain1/Ninjabrain-Bot
The triangulation, the boat-angle reconstruction and the reference measurements
in tests/ come from there.  Not affiliated with or endorsed by it, and no
replacement for it.

Modified from the original (2026-08): rewritten in Python covering only the
boat-eye path with a green boat; Wayland data-control clipboard and a unix
socket in place of AWT polling and global hotkeys; a lower bound on the
ray-wedge tolerance and a 4x4 density quadrature (see stronghold.triangulate);
float32 snapping dropped from the boat angle grid as a verified no-op.

Copyright (C) 2026 Ktrompfl.  Free software under the GNU General Public
License, version 3; see LICENSE.  No warranty, to the extent permitted by law.
"""

from sh_calc.daemon import Command, Level, Session, State, parse_command, parse_f3c
from sh_calc.stronghold import (
    Config,
    Dimension,
    Position,
    Prediction,
    Throw,
    make_throw,
    triangulate,
)

__all__ = [
    "Command",
    "Config",
    "Dimension",
    "Level",
    "Position",
    "Prediction",
    "Session",
    "State",
    "Throw",
    "make_throw",
    "parse_command",
    "parse_f3c",
    "triangulate",
]
