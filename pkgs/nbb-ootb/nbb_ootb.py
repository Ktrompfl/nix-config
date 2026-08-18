"""Ninjabrain Bot, out of the box.

The bot is an X11 program: it polls the X clipboard and grabs global hotkeys
with XRecord, neither of which works under Wayland.  So it gets an X server of
its own -- a rootful XWayland with no other client, where the clipboard can be
pushed in with `wl-paste --watch` and keys injected over XTEST, both of which
work regardless of focus.
"""

from __future__ import annotations

import argparse
import json
import os
import signal
import subprocess
import sys
import time
from collections.abc import Callable

from Xlib import X, display
from Xlib.ext import xtest

# Baked in at build time by the home-manager module.
with open("@config@") as _config_file:
    _CONFIG = json.load(_config_file)

DISPLAY: str = _CONFIG["display"]
GEOMETRY: str = _CONFIG["geometry"]

#: Per action, the X keycode of its hotkey and of the modifiers held with it.
ACTIONS: dict[str, tuple[int, list[int]]] = {
    action: (key["keycode"], key["modifiers"])
    for action, key in _CONFIG["actions"].items()
}

#: How we tell the box is up.
SOCKET = "/tmp/.X11-unix/X" + DISPLAY.lstrip(":").split(".")[0]


def wait_for(predicate: Callable[[], bool], timeout: float) -> bool:
    """Poll `predicate` until it is true, or give up after `timeout` seconds."""
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if predicate():
            return True
        time.sleep(0.1)
    return False


def start() -> None:
    """Run the bot inside its own X server, fed by the Wayland clipboard."""
    if os.path.exists(SOCKET):
        sys.exit(f"nbb-ootb: display {DISPLAY} is already in use")

    # Unwind through `finally`, or the box outlives its session.
    signal.signal(signal.SIGTERM, lambda *_: sys.exit(0))

    children: list[subprocess.Popen[bytes]] = []
    try:
        # Rootful (no -rootless): one Wayland window holding the X root.
        children.append(subprocess.Popen(["Xwayland", DISPLAY, "-geometry", GEOMETRY]))

        if not wait_for(lambda: os.path.exists(SOCKET), 10):
            sys.exit(f"nbb-ootb: Xwayland did not come up on {DISPLAY}")

        # wl-paste runs a child per copy, which hands the entry to the box.
        # xclip forks off to serve it and exits once the next one takes over.
        children.append(
            subprocess.Popen(
                [
                    "wl-paste",
                    "--type",
                    "text",
                    "--watch",
                    "xclip",
                    "-display",
                    DISPLAY,
                    "-selection",
                    "clipboard",
                    "-i",
                ]
            )
        )

        bot = subprocess.Popen(
            ["ninjabrain-bot"], env=dict(os.environ, DISPLAY=DISPLAY)
        )
        children.append(bot)
        sys.exit(bot.wait())
    finally:
        for child in reversed(children):
            child.terminate()


def send(actions: list[str]) -> None:
    """Replay the keys of some actions into the display the bot is on."""
    unknown = [action for action in actions if action not in ACTIONS]
    if unknown:
        sys.exit(f"nbb-ootb: no hotkey configured for: {', '.join(unknown)}")
    if not os.path.exists(SOCKET):
        sys.exit(f"nbb-ootb: nothing running on {DISPLAY}")

    conn = display.Display(DISPLAY)
    for action in actions:
        keycode, modifiers = ACTIONS[action]
        # Held around the key: the bot matches on the reported modifiers.
        for modifier in modifiers:
            xtest.fake_input(conn, X.KeyPress, modifier)
        xtest.fake_input(conn, X.KeyPress, keycode)
        xtest.fake_input(conn, X.KeyRelease, keycode)
        for modifier in reversed(modifiers):
            xtest.fake_input(conn, X.KeyRelease, modifier)
        conn.sync()


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Run Ninjabrain Bot, or send it one of its hotkeys.",
        epilog="With no arguments, starts the bot in an X server of its own.",
    )
    parser.add_argument("actions", nargs="*", metavar="ACTION", help="hotkeys to send")
    parser.add_argument(
        "--list", action="store_true", help="list the configured actions"
    )
    args = parser.parse_args()

    if args.list:
        for action in sorted(ACTIONS):
            print(action)
    elif args.actions:
        send(args.actions)
    else:
        start()


if __name__ == "__main__":
    main()
