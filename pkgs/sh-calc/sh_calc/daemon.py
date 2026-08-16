"""The program: clipboard subscription, session state, command socket and CLI.

A daemon is not needed for speed -- a cold start is ~100 ms, almost all of it
importing numpy.  It exists because something has to hold the clipboard
subscription open, and because the workflow is stateful across events:
corrections accumulate on the last throw and undo walks back through history.
State is an immutable snapshot, so undo/redo is just two stacks.

Output goes two ways: printed for the terminal it runs in, and written to a
status file for the waywall overlay to poll.  `report` splits it into
severity-tagged lines rather than one blob of text, because the overlay draws
each line as its own object and colours it by severity.
"""

from __future__ import annotations

import argparse
import math
import os
import queue
import signal
import socket
import subprocess
import sys
import threading
import time
from dataclasses import dataclass, replace
from enum import StrEnum

from sh_calc.stronghold import (
    Config,
    Dimension,
    Position,
    Prediction,
    Throw,
    make_throw,
    triangulate,
)

# ==== READING THE CLIPBOARD =============================================


def parse_f3c(text: str) -> Position | None:
    """Parse `/execute in <dim> run tp @s X Y Z YAW PITCH`."""
    match text.split(" "):
        case ["/execute", "in", world, "run", "tp", "@s", x, y, z, yaw, pitch]:
            dimension = Dimension.from_namespaced_id(world)
            if dimension is None:
                return None
            try:
                values = [float(v) for v in (x, y, z, yaw, pitch)]
            except ValueError:
                return None
            return Position(*values, dimension=dimension)
        case _:
            return None


# ==== SESSION STATE =====================================================


class Level(StrEnum):
    """How a line of the report should be shown."""

    HEAD = "head"  #: column headers
    MAIN = "main"  #: the answer to act on
    DIM = "dim"  #: alternatives and detail
    WARN = "warn"  #: usable, but something is off
    ERROR = "error"  #: no usable answer


#: A report line, or a note about what just happened.
type Line = tuple[Level, str]


class Command(StrEnum):
    """An action a compositor keybind can trigger, sent to the daemon by name."""

    description: str

    def __new__(cls, value: str, description: str) -> Command:
        member = str.__new__(cls, value)
        member._value_ = value
        member.description = description
        return member

    RESET = "reset", "clear all throws"
    UNDO = "undo", "step back through the history"
    REDO = "redo", "step forward again"
    INC = "inc", "nudge the last throw one subpixel clockwise"
    DEC = "dec", "nudge the last throw one subpixel anticlockwise"


def parse_command(text: str) -> Command | None:
    """Validate a name arriving over the socket."""
    try:
        return Command(text)
    except ValueError:
        return None


@dataclass(frozen=True)
class State:
    throws: tuple[Throw, ...] = ()


_PREDICTION_HEADER = "  chunk         nether        dist       %"
_THROW_HEADER = "  throw        x       z     angle          error"


class Session:
    """Throw list, with undo/redo over immutable snapshots."""

    def __init__(self, cfg: Config):
        self.cfg: Config = cfg
        self.state: State = State()
        self.player: Position | None = None
        self._undo: list[State] = []
        self._redo: list[State] = []

    def _apply(self, new: State) -> None:
        self._undo.append(self.state)
        self._redo.clear()
        self.state = new

    def command(self, cmd: Command) -> Line | None:
        """Apply a keybind action.  Returns a note to show, if any."""
        s = self.state
        match cmd:
            case Command.RESET:
                self._apply(State())
                return (Level.DIM, "reset")

            case Command.INC | Command.DEC:
                if not s.throws:
                    return (Level.WARN, "no throw to correct")
                last = s.throws[-1]
                step = 1 if cmd is Command.INC else -1
                nudged = replace(last, increments=last.increments + step)
                self._apply(replace(s, throws=s.throws[:-1] + (nudged,)))
                return None

            case Command.UNDO:
                if not self._undo:
                    return (Level.WARN, "nothing to undo")
                self._redo.append(self.state)
                self.state = self._undo.pop()
                return None

            case Command.REDO:
                if not self._redo:
                    return (Level.WARN, "nothing to redo")
                self._undo.append(self.state)
                self.state = self._redo.pop()
                return None

    def position(self, pos: Position) -> Line | None:
        """Feed an F3+C reading.  Returns a note to show, if any."""
        self.player = pos  # tracked outside the undo history; it is only a readout
        s = self.state

        if pos.dimension is not Dimension.OVERWORLD:
            return None
        if pos.looking_below_horizon:
            return None

        throw = make_throw(pos, self.cfg)
        if s.throws and (s.throws[-1].x, s.throws[-1].z, s.throws[-1].base) == (
            throw.x,
            throw.z,
            throw.base,
        ):
            return None  # same throw again
        self._apply(replace(s, throws=s.throws + (throw,)))
        return None

    def _prediction_row(self, p: Prediction) -> str:
        chunk = f"{p.cx}, {p.cz}"
        nether = f"{p.x // 8}, {p.z // 8}"
        distance = p.distance(self.player) if self.player else 0
        return f"  {chunk:<14}{nether:<14}{distance:>4}  {p.certainty * 100:>5.1f}%"

    def _throw_row(self, index: int, throw: Throw, best: Prediction) -> Line:
        """One measurement, with how far off the answer it lands.

        Flagged once the residual is too big to blame on either the model or the
        screen: three sigma, but never less than one subpixel, since that is the
        finest correction there is.  Ninjabrain-Bot's boat integration tests
        assert the same subpixel bound on their measurements.
        """
        error = best.angle_error(throw, self.cfg)
        tolerance = max(throw.step, 3 * best.angle_sigma(throw, self.cfg))
        level = Level.WARN if abs(error) > tolerance else Level.DIM
        # The corrections trail the angle rather than getting a column of their
        # own: they are a property of that angle, not a separate measurement.
        steps = f" {throw.increments:+d}" if throw.increments else ""
        return (
            level,
            f"  {index:<7}{math.floor(throw.x):>7}{math.floor(throw.z):>8}"
            f"{throw.angle:>10.3f}{steps:<5}{error:>+10.4f}",
        )

    def report(self) -> list[Line]:
        """The whole display, one line at a time, most important first."""
        throws = list(self.state.throws)
        if not throws:
            return [(Level.DIM, "no throws")]

        predictions = triangulate(throws, self.cfg)
        if not predictions or not predictions[0].ok:
            return [(Level.ERROR, f"no prediction ({len(throws)} throws)")]

        best = predictions[0]
        lines: list[Line] = [(Level.HEAD, _PREDICTION_HEADER)]
        lines.append(
            (
                Level.WARN if best.certainty < 0.5 else Level.MAIN,
                self._prediction_row(best),
            )
        )
        if best.certainty < 0.95:
            for alt in predictions[1:4]:
                if alt.certainty < 0.001:
                    break
                lines.append((Level.DIM, self._prediction_row(alt)))

        lines.append((Level.HEAD, _THROW_HEADER))
        lines.extend(
            self._throw_row(i, throw, best) for i, throw in enumerate(throws, 1)
        )
        return lines

    def render(self) -> str:
        """The report as plain text, for the terminal the daemon runs in."""
        return "\n".join(text for _, text in self.report())


# ==== THE RUNNING DAEMON ================================================


RUNTIME_DIR = os.environ.get("XDG_RUNTIME_DIR", "/tmp")
SOCKET_PATH = os.path.join(RUNTIME_DIR, "sh-calc.sock")
STATUS_PATH = os.path.join(RUNTIME_DIR, "sh-calc.status")

type Events = queue.Queue[tuple[str, str]]

#: A wl-paste that dies this fast never got as far as watching anything, so the
#: fault is with the compositor rather than the subscription.
STARTUP_GRACE_SECONDS = 2.0
#: Give up after this many immediate failures in a row.  Retrying forever turns
#: a compositor without data-control into an endless connect/disconnect loop in
#: someone else's log, which is exactly what this used to do.
MAX_RAPID_FAILURES = 3


def watch_clipboard(events: Events) -> None:
    """Feed clipboard changes into `events` using the data-control protocol.

    wl-paste pipes each new selection to a child process with no delimiter
    between events, so the child appends a newline.  F3+C strings are always a
    single line, and everything not starting with `/execute in ` is ignored, so
    line-based framing is safe.
    """
    failures = 0
    reason = ""
    while failures < MAX_RAPID_FAILURES:
        started = time.monotonic()
        proc = subprocess.Popen(
            ["wl-paste", "--watch", "sh", "-c", "cat; echo"],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        assert proc.stdout is not None and proc.stderr is not None
        first = True
        for raw in proc.stdout:
            line = raw.decode("utf-8", "replace").strip()
            if first:
                first = False  # wl-paste replays the current clipboard on startup
                continue
            if line:
                events.put(("clipboard", line))
        _ = proc.wait()
        reason = proc.stderr.read().decode("utf-8", "replace").strip()

        # A watch that ran for a while and then died is worth retrying; one that
        # never started is not going to start on the next attempt either.
        died_at_once = time.monotonic() - started < STARTUP_GRACE_SECONDS
        failures = failures + 1 if died_at_once else 0
        if failures < MAX_RAPID_FAILURES:
            print(f"wl-paste died, restarting in 1s: {reason}", file=sys.stderr)
            time.sleep(1)
    # The whole of wl-paste's complaint went to the log above; the overlay gets
    # the first line, which is the error rather than the advice that follows it.
    first_line = next((line for line in reason.splitlines() if line), "")
    events.put(("fatal", first_line or "wl-paste exited immediately"))


def claim_socket(path: str) -> None:
    """Remove a stale socket, but refuse to displace a daemon that is still live."""
    if not os.path.exists(path):
        return
    try:
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as probe:
            probe.connect(path)
    except (ConnectionRefusedError, FileNotFoundError):
        os.unlink(path)
    else:
        raise SystemExit(f"a daemon is already listening on {path}")


def serve_commands(events: Events, path: str) -> None:
    server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    server.bind(path)
    server.listen(8)
    while True:
        conn, _ = server.accept()
        with conn:
            cmd = conn.recv(64).decode("utf-8", "replace").strip()
        if cmd:
            events.put(("command", cmd))


def send_command(cmd: str, path: str) -> None:
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as sock:
        sock.connect(path)
        sock.sendall(cmd.encode())


def write_status(path: str, lines: list[Line]) -> None:
    """Publish the report as `<level>|<text>` lines, one line per display row.

    Written to a temporary file and renamed, so a poller never sees half of an
    update.
    """
    body = "".join(f"{level}|{' '.join(text.splitlines())}\n" for level, text in lines)
    tmp = f"{path}.tmp"
    with open(tmp, "w", encoding="utf-8") as out:
        _ = out.write(body)
    os.replace(tmp, path)


def run_daemon(cfg: Config, path: str, status_path: str) -> int:
    claim_socket(path)
    _ = signal.signal(signal.SIGTERM, lambda *_: sys.exit(0))

    events: Events = queue.Queue()
    threading.Thread(target=watch_clipboard, args=(events,), daemon=True).start()
    threading.Thread(target=serve_commands, args=(events, path), daemon=True).start()

    session = Session(cfg)
    print(f"listening on {path}, watching the clipboard", flush=True)
    write_status(status_path, session.report())
    keep_status = False
    try:
        while True:
            kind, payload = events.get()
            before = session.state
            note: Line | None
            match kind:
                case "clipboard":
                    pos = parse_f3c(payload)
                    if pos is None:
                        continue
                    note = session.position(pos)
                case "fatal":
                    # Leave the status file behind so the overlay keeps showing
                    # why it stopped instead of silently going blank.
                    message = f"clipboard watch failed: {payload}"
                    write_status(status_path, [(Level.ERROR, message)])
                    keep_status = True
                    print(message, file=sys.stderr)
                    return 1
                case _:
                    cmd = parse_command(payload)
                    if cmd is None:
                        note = (Level.ERROR, f"unknown command: {payload}")
                    else:
                        note = session.command(cmd)

            # Stay quiet when nothing happened -- a repeated F3+C, or a
            # duplicate selection event from the clipboard owner.
            if not note and session.state is before:
                continue
            report = session.report()
            write_status(status_path, ([note] if note else []) + report)
            print()
            if note:
                print(f"  {note[1]}")
            print("\n".join(text for _, text in report))
            sys.stdout.flush()
    except (KeyboardInterrupt, SystemExit):
        pass
    finally:
        stale = [path] if keep_status else [path, status_path]
        for leftover in stale:
            if os.path.exists(leftover):
                os.unlink(leftover)
    return 0


def build_parser() -> argparse.ArgumentParser:
    defaults = Config()
    parser = argparse.ArgumentParser(
        prog="sh-calc",
        description="Stronghold calculator for 1.16 boat eye.",
        epilog="Run `sh-calc daemon` in a terminal; bind the other commands in your "
        "compositor.",
    )
    shared = argparse.ArgumentParser(add_help=False)
    shared.add_argument(
        "--socket", default=SOCKET_PATH, help="daemon socket (default: %(default)s)"
    )
    sub = parser.add_subparsers(dest="command", required=True, metavar="<command>")

    daemon = sub.add_parser(
        "daemon", parents=[shared], help="watch the clipboard and print predictions"
    )
    daemon.add_argument(
        "--status",
        default=STATUS_PATH,
        help="file the overlay polls for the report (default: %(default)s)",
    )
    daemon.add_argument(
        "--sensitivity",
        type=float,
        default=defaults.sensitivity,
        help="Minecraft raw mouse sensitivity (default: %(default)s)",
    )
    daemon.add_argument(
        "--sigma-boat",
        type=float,
        default=defaults.sigma_boat,
        help="measurement error in degrees (default: %(default)s)",
    )
    daemon.add_argument(
        "--crosshair-correction",
        type=float,
        default=defaults.crosshair_correction,
        help="per-user crosshair calibration in degrees (default: %(default)s)",
    )
    daemon.add_argument(
        "--chunk-offset",
        type=int,
        choices=(0, 8),
        default=defaults.chunk_offset,
        help="8 for Minecraft before 1.19, 0 from 1.19 on (default: %(default)s)",
    )

    for cmd in Command:
        _ = sub.add_parser(cmd.value, parents=[shared], help=cmd.description)
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)

    if args.command == "daemon":
        return run_daemon(
            Config(
                sensitivity=args.sensitivity,
                sigma_boat=args.sigma_boat,
                crosshair_correction=args.crosshair_correction,
                chunk_offset=args.chunk_offset,
            ),
            args.socket,
            args.status,
        )

    try:
        send_command(Command(args.command).value, args.socket)
    except (FileNotFoundError, ConnectionRefusedError):
        print(f"no daemon running (socket {args.socket})", file=sys.stderr)
        return 1
    return 0
