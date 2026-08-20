"""Ninjabrain Bot, with its actions reachable from outside the program.

A java agent loaded into the bot's JVM listens on a unix socket and triggers
the bot's own actions off it, so anything that can bind a key -- a compositor,
a window manager, a script -- can drive it without the bot having to see the
key itself. This is the other end of that socket.

That indirection is what makes the bot usable under Wayland at all, where it
can neither grab a global hotkey nor read the clipboard; there the agent also
feeds it the clipboard. Everywhere else the bot is untouched, and this is only
an extra way in.
"""

from __future__ import annotations

import argparse
import os
import socket
import sys

#: The bot itself, which the agent is already wrapped around.
BOT = "@ninjabrain-bot@"

#: Where the agent listens. Kept in step with `Agent.socketPath`.
SOCKET = os.path.join(os.environ.get("XDG_RUNTIME_DIR", "/tmp"), "ninjabrain-bot.sock")


def ask(request: str) -> str:
    """Send `request` to the running bot and return what it answers."""
    try:
        conn = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        conn.connect(SOCKET)
    except OSError as error:
        sys.exit(f"ninjabrain-bot: no bot on {SOCKET}: {error}")
    with conn:
        conn.sendall(request.encode())
        conn.shutdown(socket.SHUT_WR)
        answer = b"".join(iter(lambda: conn.recv(4096), b"")).decode().strip()
    if answer.startswith("error: "):
        sys.exit(f"ninjabrain-bot: {answer.removeprefix('error: ')}")
    return answer


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Run Ninjabrain Bot, or trigger one of its actions.",
        epilog="With no arguments, starts the bot.",
    )
    parser.add_argument("actions", nargs="*", metavar="ACTION", help="actions to trigger")
    parser.add_argument("--list", action="store_true", help="list the bound actions")
    args = parser.parse_args()

    if args.list:
        print(ask("list\n"))
    elif args.actions:
        ask("run " + " ".join(args.actions) + "\n")
    else:
        # The wrapper has already put the agent in JDK_JAVA_OPTIONS.
        os.execv(BOT, [BOT])


if __name__ == "__main__":
    main()
