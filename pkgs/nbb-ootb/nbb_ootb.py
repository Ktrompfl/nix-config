"""Ninjabrain Bot, out of the box.

The bot is an X11 program: it polls the X clipboard and grabs global hotkeys
with XRecord, neither of which works under Wayland.  So it gets an X server of
its own -- a rootful XWayland with no other client, where the clipboard can be
pushed in with `wl-paste --watch` and keys injected over XTEST, both of which
work regardless of focus.  Only what the bot can parse is passed in.  --bare skips the box for waywall, which already has
an X server the game and the bot can share.
"""

from __future__ import annotations

import argparse
import json
import os
import pathlib
import shutil
import signal
import stat
import subprocess
import sys
import threading
import time
from collections.abc import Callable

from Xlib import X, display
from Xlib.ext import xtest

#: An X keycode, and the keycodes of any modifiers to hold with it.
type Key = tuple[int, list[int]]

# Baked in at build time by the home-manager module.
with open("@config@") as _config_file:
    _CONFIG = json.load(_config_file)

DISPLAY: str = _CONFIG["display"]
GEOMETRY: str = _CONFIG["geometry"]
PREFS: str | None = _CONFIG["prefs"]
PREFS_PATH: str = _CONFIG["prefsPath"]
BOT: str = _CONFIG["bot"]
XWAYLAND: str = _CONFIG["xwayland"]
WL_PASTE: str = _CONFIG["wlPaste"]
XCLIP: str = _CONFIG["xclip"]
SHELL: str = _CONFIG["shell"]

ACTIONS: dict[str, Key] = {
    action: (key["keycode"], key["modifiers"]) for action, key in _CONFIG["actions"].items()
}


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
    is held back.  The last of those is bare `x z angle [correction]`, which
    any three or four numbers satisfy -- the filter is only as tight as the
    formats it has to let through.
    """
    fields = text.split(" ")
    if text.startswith("/execute in ") and len(fields) == 11:
        return fields[2].endswith(DIMENSIONS) and all(parses_as(f, float) for f in fields[6:])
    if text.startswith("/setblock ") and len(fields) == 5:
        return all(parses_as(f, int) for f in fields[1:4])
    if len(fields) in (3, 4):
        return all(parses_as(f, float) for f in fields[:3]) and (
            len(fields) == 3 or parses_as(fields[3], int)
        )
    return False


def feed_clipboard(watcher: subprocess.Popen[bytes]) -> None:
    """Hand the box the clipboard entries the bot can use, and only those.

    Everything else stays outside, so whatever is copied while the game runs is
    none of the bot's business.
    """
    assert watcher.stdout is not None
    buffered = b""
    while chunk := os.read(watcher.stdout.fileno(), 4096):
        buffered += chunk
        while b"\0" in buffered:
            entry, _, buffered = buffered.partition(b"\0")
            if is_measurement(entry.decode(errors="replace").strip()):
                _ = subprocess.run(
                    [XCLIP, "-display", DISPLAY, "-selection", "clipboard", "-i"],
                    input=entry,
                )


def socket_of(x_display: str) -> str:
    """The socket of an X display, which is how we tell it is up."""
    return "/tmp/.X11-unix/X" + x_display.lstrip(":").split(".")[0]


def install_prefs() -> None:
    """Write the declared settings where the bot reads them.

    A real file, not the store link activation puts there: the bot rewrites it
    from its own GUI.  Those changes then last until the next start.
    """
    if PREFS is None:
        return
    target = pathlib.Path(os.path.expanduser(PREFS_PATH))
    target.parent.mkdir(parents=True, exist_ok=True)
    # Activation leaves a link into the store here, which cannot be written
    # through, so replace whatever is in the way rather than opening it.
    target.unlink(missing_ok=True)
    shutil.copyfile(PREFS, target)
    os.chmod(target, stat.S_IRUSR | stat.S_IWUSR | stat.S_IRGRP | stat.S_IROTH)


def wait_for(predicate: Callable[[], bool], timeout: float) -> bool:
    """Poll `predicate` until it is true, or give up after `timeout` seconds."""
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if predicate():
            return True
        time.sleep(0.1)
    return False


def start_box() -> None:
    """Run the bot inside its own X server, fed by the Wayland clipboard."""
    x_display = DISPLAY
    socket = socket_of(x_display)
    if os.path.exists(socket):
        sys.exit(f"nbb-ootb: display {x_display} is already in use")

    # Unwind through `finally`, or the box outlives its session.
    signal.signal(signal.SIGTERM, lambda *_: sys.exit(0))

    children: list[subprocess.Popen[bytes]] = []
    try:
        # Rootful (no -rootless): one Wayland window holding the X root.
        children.append(subprocess.Popen([
            XWAYLAND, x_display, "-geometry", GEOMETRY,
        ]))

        if not wait_for(lambda: os.path.exists(socket), 10):
            sys.exit(f"nbb-ootb: Xwayland did not come up on {x_display}")

        # data-control, so it reads the selection while the game is focused.
        # The NUL is what tells one entry from the next; text cannot contain it.
        # One printf, not `cat` then `printf`: wl-paste runs a child per event,
        # a single copy in the game fires several, and they all share this pipe
        # -- two writes per entry would let them interleave into one mangled
        # entry. A lone write under PIPE_BUF cannot be split, and a measurement
        # is nowhere near that big.
        watcher = subprocess.Popen(
            [WL_PASTE, "--type", "text", "--watch", SHELL, "-c", r'printf "%s\0" "$(cat)"'],
            stdout=subprocess.PIPE,
        )
        children.append(watcher)
        threading.Thread(target=feed_clipboard, args=(watcher,), daemon=True).start()

        bot = subprocess.Popen([BOT], env=dict(os.environ, DISPLAY=x_display))
        children.append(bot)
        sys.exit(bot.wait())
    finally:
        for child in reversed(children):
            child.terminate()


def start_bare() -> None:
    """Run the bot on the ambient display, sharing it with the game.

    An injected key reaches the game too, so actions want keys it does not use,
    and the server has to allow XTEST -- waywall's does, one started with
    -enable-ei-portal (jay's own) does not.
    """
    if not os.environ.get("DISPLAY"):
        sys.exit("nbb-ootb: --bare needs a DISPLAY to run on")
    sys.exit(subprocess.Popen([BOT]).wait())


def send(actions: list[str], x_display: str) -> None:
    """Replay the keys of some actions into the display the bot is on."""
    unknown = [action for action in actions if action not in ACTIONS]
    if unknown:
        sys.exit(f"nbb-ootb: no hotkey configured for: {', '.join(unknown)}")

    conn = display.Display(x_display)
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


def target_display(requested: str | None) -> str:
    """The box if it is up, else the display we were started in (--bare)."""
    if requested:
        return requested
    if os.path.exists(socket_of(DISPLAY)):
        return DISPLAY
    ambient = os.environ.get("DISPLAY")
    if not ambient:
        sys.exit("nbb-ootb: no box running and no DISPLAY to fall back on")
    return ambient


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Run Ninjabrain Bot, or send it one of its hotkeys.",
        epilog="With no arguments, starts the bot in an X server of its own.",
    )
    parser.add_argument("actions", nargs="*", metavar="ACTION", help="hotkeys to send")
    parser.add_argument("--bare", action="store_true", help="run on the current display, without a box")
    parser.add_argument("--list", action="store_true", help="list the configured actions")
    parser.add_argument("--display", help="which display to send to")
    args = parser.parse_args()

    if args.list:
        for action in sorted(ACTIONS):
            print(action)
    elif args.actions:
        if args.bare:
            sys.exit("nbb-ootb: --bare starts the bot, it takes no actions")
        send(args.actions, target_display(args.display))
    else:
        install_prefs()
        start_bare() if args.bare else start_box()


if __name__ == "__main__":
    main()
