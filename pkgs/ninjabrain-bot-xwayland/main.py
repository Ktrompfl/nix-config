"""Ninjabrain Bot, in an XWayland of its own.

The bot is an X11 program: it polls the X clipboard and grabs global hotkeys
with XRecord, neither of which works under Wayland.  So it gets an X server of
its own -- a rootful XWayland with no other client, where the clipboard can be
pushed in with `wl-paste --watch` and keys injected over XTEST, both of which
work regardless of focus.

How big that server is and which key stands for which action are read out of
the bot's own settings, so there is nothing to keep in sync with them.
"""

from __future__ import annotations

import argparse
import os
import pathlib
import signal
import subprocess
import sys
import time
from collections.abc import Callable
from xml.etree import ElementTree

from Xlib import X
from Xlib.display import Display
from Xlib.ext import xtest

#: Where java.util.prefs keeps what the bot has been told.
PREFS = pathlib.Path.home() / ".java/.userPrefs/ninjabrainbot/prefs.xml"

#: Command line action, and the preference its hotkey is stored under.
ACTIONS = {
    "increment": "hotkey_increment",
    "decrement": "hotkey_decrement",
    "reset": "hotkey_reset",
    "undo": "hotkey_undo",
    "redo": "hotkey_redo",
    "minimize": "hotkey_minimize",
    "alt-std": "hotkey_alt_std",
    "lock": "hotkey_lock",
    "boat": "hotkey_boat",
    "mod-360": "hotkey_mod_360",
    "aa-mode": "hotkey_toggle_aa_mode",
}

#: JNativeHook's modifier masks, and the X keycode that holds each down.
MODIFIERS = {1: 50, 2: 37, 4: 133, 8: 64, 16: 62, 32: 105, 64: 134, 128: 108}

#: The bot's window at each `size` and `view`, measured on 1.5.2. Anything it
#: is not sized for lands on the largest, which only ever leaves a margin.
WINDOWS = {
    (0, 0): "320x202",
    (0, 1): "420x236",
    (1, 0): "380x222",
    (1, 1): "490x262",
    (2, 0): "570x301",
    (2, 1): "730x358",
}


def socket_of(display: str) -> str:
    """The socket of an X display, which is how we tell it is up."""
    return "/tmp/.X11-unix/X" + display.lstrip(":").split(".")[0]


def settings() -> dict[str, str]:
    """What the bot has stored, as flat key/value pairs."""
    try:
        entries = ElementTree.parse(PREFS).iter("entry")
    except (OSError, ElementTree.ParseError) as error:
        sys.exit(f"ninjabrain-bot-xwayland: cannot read {PREFS}: {error}")
    return {entry.get("key", ""): entry.get("value", "") for entry in entries}


def geometry(stored: dict[str, str]) -> str:
    """How big the box has to be for the window the bot will open in it."""
    shape = (int(stored.get("size", 0)), int(stored.get("view", 0)))
    return WINDOWS.get(shape, WINDOWS[max(WINDOWS)])


def hotkey(stored: dict[str, str], action: str) -> tuple[int, list[int]]:
    """The X keycode of an action's hotkey, and those of its modifiers.

    The bot stores `code | location << 16`, where code is the evdev code for
    everything in the main key block -- eight below the X keycode. Extended
    keys (arrows, numpad) do not follow that rule and cannot be sent.
    """
    code = stored.get(f"{ACTIONS[action]}_code")
    if code is None:
        sys.exit(f"ninjabrain-bot-xwayland: {action} has no hotkey")
    mask = int(stored.get(f"{ACTIONS[action]}_modifier", 0))
    return (
        int(code) % 65536 + 8,
        [key for bit, key in MODIFIERS.items() if mask & bit],
    )


def wait_for(predicate: Callable[[], bool], timeout: float) -> bool:
    """Poll `predicate` until it is true, or give up after `timeout` seconds."""
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if predicate():
            return True
        time.sleep(0.1)
    return False


def start(display: str) -> None:
    """Run the bot inside its own X server, fed by the Wayland clipboard."""
    socket = socket_of(display)
    if os.path.exists(socket):
        sys.exit(f"ninjabrain-bot-xwayland: display {display} is already in use")

    # Unwind through `finally`, or the box outlives its session.
    signal.signal(signal.SIGTERM, lambda *_: sys.exit(0))

    children: list[subprocess.Popen[bytes]] = []
    try:
        # Rootful (no -rootless): one Wayland window holding the X root.
        children.append(
            subprocess.Popen(["Xwayland", display, "-geometry", geometry(settings())])
        )

        if not wait_for(lambda: os.path.exists(socket), 10):
            sys.exit(f"ninjabrain-bot-xwayland: Xwayland did not come up on {display}")

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
                    display,
                    "-selection",
                    "clipboard",
                    "-i",
                ]
            )
        )

        bot = subprocess.Popen(
            ["ninjabrain-bot"], env=dict(os.environ, DISPLAY=display)
        )
        children.append(bot)
        sys.exit(bot.wait())
    finally:
        for child in reversed(children):
            child.terminate()


def send(actions: list[str], display: str) -> None:
    """Replay the hotkeys of some actions into the display the bot is on."""
    unknown = [action for action in actions if action not in ACTIONS]
    if unknown:
        sys.exit(f"ninjabrain-bot-xwayland: no such action: {', '.join(unknown)}")
    if not os.path.exists(socket_of(display)):
        sys.exit(f"ninjabrain-bot-xwayland: nothing running on {display}")

    stored = settings()
    conn = Display(display)
    for action in actions:
        keycode, modifiers = hotkey(stored, action)
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
    parser.add_argument("--display", default=":77", help="the display the box runs on")
    parser.add_argument("--list", action="store_true", help="list the bound actions")
    args = parser.parse_args()

    if args.list:
        stored = settings()
        for action, preference in sorted(ACTIONS.items()):
            if f"{preference}_code" in stored:
                print(action)
    elif args.actions:
        send(args.actions, args.display)
    else:
        start(args.display)


if __name__ == "__main__":
    main()
