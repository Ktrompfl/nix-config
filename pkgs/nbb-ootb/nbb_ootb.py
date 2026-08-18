"""Ninjabrain Bot, out of the box.

The bot is an X11 program: it polls the X clipboard and grabs global hotkeys
with XRecord, neither of which works under Wayland.  So it gets an X server of
its own -- a rootful XWayland with no other client, where the clipboard can be
pushed in with `wl-paste --watch` and keys injected over XTEST, both of which
work regardless of focus.  Only what the bot can parse is passed in.
"""

from __future__ import annotations

import argparse
import json
import os
import signal
import subprocess
import sys
import threading
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

#: The dimensions the bot recognises in an F3+C string.
DIMENSIONS = ("overworld", "the_nether", "the_end")


def parses_as(text: str, parse: Callable[[str], object]) -> bool:
    try:
        _ = parse(text)
    except ValueError:
        return False
    return True


def is_measurement(text: str) -> bool:
    """Whether the bot would read anything out of this clipboard entry.

    Mirrors F3CData, F3IData and InputData1_12, so nothing the bot understands
    is held back.  The last of those is plain `x z angle [correction]`, which
    any three or four numbers satisfy -- the filter is only as tight as the
    formats it has to let through.
    """
    fields = text.split(" ")
    if text.startswith("/execute in ") and len(fields) == 11:
        return fields[2].endswith(DIMENSIONS) and all(
            parses_as(f, float) for f in fields[6:]
        )
    if text.startswith("/setblock ") and len(fields) == 5:
        return all(parses_as(f, int) for f in fields[1:4])
    if len(fields) in (3, 4):
        return all(parses_as(f, float) for f in fields[:3]) and (
            len(fields) == 3 or parses_as(fields[3], int)
        )
    return False


def feed_clipboard(watcher: subprocess.Popen[bytes]) -> None:
    """Hand the box the clipboard entries the bot can use, and only those."""
    assert watcher.stdout is not None
    buffered = b""
    while chunk := os.read(watcher.stdout.fileno(), 4096):
        buffered += chunk
        while b"\0" in buffered:
            entry, _, buffered = buffered.partition(b"\0")
            if is_measurement(entry.decode(errors="replace").strip()):
                print(f"sending clipboard: {entry}")
                _ = subprocess.run(
                    ["xclip", "-display", DISPLAY, "-selection", "clipboard", "-i"],
                    input=entry,
                )


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

        # The NUL is what tells one entry from the next; text cannot contain it.
        # One printf, not `cat` then `printf`: wl-paste runs a child per event,
        # a single copy in the game fires several, and they all share this pipe
        # -- two writes per entry would let them interleave into one mangled
        # entry. A lone write under PIPE_BUF cannot be split, and a measurement
        # is nowhere near that big.
        watcher = subprocess.Popen(
            [
                "wl-paste",
                "--type",
                "text",
                "--watch",
                "bash",
                "-c",
                r'printf "%s\0" "$(cat)"',
            ],
            stdout=subprocess.PIPE,
        )
        children.append(watcher)
        threading.Thread(target=feed_clipboard, args=(watcher,), daemon=True).start()

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
