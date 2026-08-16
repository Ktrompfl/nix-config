"""The program: session state, the report, argument parsing and supervision."""

from __future__ import annotations

import os
import queue
import tempfile
import unittest
from collections.abc import Iterator
from unittest import mock

from sh_calc import daemon
from sh_calc.daemon import (
    Command,
    Level,
    State,
    build_parser,
    parse_command,
    parse_f3c,
)
from sh_calc.stronghold import Dimension
from tests.cases import CASES, fresh, parse, play


class F3CParsing(unittest.TestCase):
    OVERWORLD: str = (
        "/execute in minecraft:overworld run tp @s 1.5 64.0 -2.5 -78.75 32.82"
    )

    def test_parses_overworld(self):
        pos = parse(self.OVERWORLD)
        self.assertEqual(
            (pos.x, pos.y, pos.z, pos.yaw, pos.pitch),
            (1.5, 64.0, -2.5, -78.75, 32.82),
        )
        self.assertIs(pos.dimension, Dimension.OVERWORLD)
        self.assertEqual(pos.scale, 1.0)

    def test_nether_coordinates_scale_by_eight(self):
        pos = parse(self.OVERWORLD.replace("overworld", "the_nether"))
        self.assertIs(pos.dimension, Dimension.NETHER)
        self.assertEqual(pos.scale, 8.0)

    def test_parses_the_end(self):
        pos = parse(self.OVERWORLD.replace("overworld", "the_end"))
        self.assertIs(pos.dimension, Dimension.END)

    def test_rejects_non_f3c_input(self):
        for text in [
            "",
            "hello world",
            "/execute in minecraft:overworld run tp @s 1.5 64.0 -2.5 -78.75",  # short
            self.OVERWORLD + " extra",
            self.OVERWORLD.replace("64.0", "sixty"),
            self.OVERWORLD.replace("minecraft:overworld", "minecraft:moon"),
        ]:
            with self.subTest(text=text):
                self.assertIsNone(parse_f3c(text))


class SessionBehaviour(unittest.TestCase):
    def test_an_f3c_reading_becomes_a_throw(self):
        session = fresh()
        _ = session.position(parse(CASES[0].eye_f3c))
        self.assertEqual(len(session.state.throws), 1)

    def test_repeated_identical_throw_is_ignored(self):
        case = CASES[0]
        session = fresh()
        _ = session.position(parse(case.eye_f3c))
        _ = session.position(parse(case.eye_f3c))
        self.assertEqual(len(session.state.throws), 1)

    def test_looking_below_horizon_is_not_a_throw(self):
        session = fresh()
        _ = session.position(
            parse("/execute in minecraft:overworld run tp @s 0.0 64.0 0.0 -78.75 12.0")
        )
        self.assertEqual(session.state.throws, ())

    def test_other_dimensions_are_not_throws(self):
        session = fresh()
        _ = session.position(
            parse("/execute in minecraft:the_nether run tp @s 0.0 64.0 0.0 -78.75 -12.0")
        )
        self.assertEqual(session.state.throws, ())

    def test_undo_and_redo_round_trip(self):
        session = play(CASES[0])
        before = session.state
        _ = session.command(Command.INC)
        self.assertNotEqual(session.state, before)
        _ = session.command(Command.UNDO)
        self.assertEqual(session.state, before)
        _ = session.command(Command.REDO)
        self.assertNotEqual(session.state, before)

    def test_inc_and_dec_cancel(self):
        session = play(CASES[0])
        before = session.state.throws[-1].angle
        _ = session.command(Command.INC)
        _ = session.command(Command.DEC)
        self.assertAlmostEqual(session.state.throws[-1].angle, before, places=12)

    def test_undo_on_empty_history_is_reported(self):
        self.assertEqual(fresh().command(Command.UNDO), (Level.WARN, "nothing to undo"))

    def test_redo_without_undo_is_reported(self):
        self.assertEqual(fresh().command(Command.REDO), (Level.WARN, "nothing to redo"))

    def test_correction_without_a_throw_is_reported(self):
        self.assertEqual(
            fresh().command(Command.INC), (Level.WARN, "no throw to correct")
        )

    def test_reset_clears_the_throws(self):
        session = play(CASES[0])
        _ = session.command(Command.RESET)
        self.assertEqual(session.state, State())

    def test_unknown_command_names_are_rejected_at_the_boundary(self):
        self.assertIsNone(parse_command("wat"))
        self.assertIs(parse_command("reset"), Command.RESET)

    def test_boat_is_no_longer_a_command(self):
        """The yaw grid is assumed to sit at 0, so there is nothing to measure."""
        self.assertIsNone(parse_command("boat"))
        self.assertIsNone(parse_command("mod360"))


class Report(unittest.TestCase):
    def test_no_throws(self):
        self.assertEqual(fresh().report(), [(Level.DIM, "no throws")])

    def test_includes_the_prediction_and_the_throw(self):
        report = play(CASES[0]).report()
        levels = [level for level, _ in report]
        text = "\n".join(line for _, line in report)
        self.assertIn("133, -14", text)
        self.assertIn("266, -28", text)
        self.assertIn(Level.HEAD, levels)
        self.assertIn(Level.MAIN, levels)

    @staticmethod
    def throw_fields(session) -> list[str]:
        """The last throw row: index, x, z, angle, [adjustment], error."""
        return session.report()[-1][1].split()

    def test_the_throw_row_shows_where_it_was_made(self):
        self.assertEqual(self.throw_fields(play(CASES[0]))[1:3], ["1275", "1064"])

    def test_the_corrections_ride_along_with_the_angle(self):
        """The one number you cannot recover by looking at the game: how many
        subpixels you have already nudged, and which way."""
        session = play(CASES[0])  # seven increments
        self.assertEqual(self.throw_fields(session)[3:5], ["-146.099", "+7"])

        for _step in range(9):
            _ = session.command(Command.DEC)
        self.assertEqual(self.throw_fields(session)[4], "-2")

    def test_an_uncorrected_throw_shows_no_adjustment(self):
        fields = self.throw_fields(play(CASES[3]))  # no corrections
        self.assertEqual(len(fields), 5, fields)

    def test_a_well_corrected_throw_is_not_flagged(self):
        for i, case in enumerate(CASES, 1):
            with self.subTest(case=i):
                report = play(case).report()
                self.assertNotIn(Level.WARN, [level for level, _ in report])

    def test_a_badly_corrected_throw_is_flagged(self):
        """Ten subpixels past the right answer, no chunk fits the ray, and the
        throw row says so.  Over-correction does not always land this way -- one
        ray usually has some chunk near it -- so this pins a case that does."""
        session = play(CASES[0])
        for _step in range(10):
            _ = session.command(Command.INC)
        flagged = [text for level, text in session.report() if level is Level.WARN]
        self.assertEqual(len(flagged), 1)
        self.assertTrue(flagged[0].startswith("  1 "), flagged[0])

    def test_a_confident_prediction_leads_the_report(self):
        report = play(CASES[0]).report()
        self.assertIs(report[0][0], Level.HEAD)
        self.assertIs(report[1][0], Level.MAIN)

    def test_a_coin_flip_prediction_is_flagged(self):
        """Case 3 nudged ten subpixels too far lands at ~45%: still the top
        chunk, but not one to walk 2000 blocks towards without noticing."""
        session = play(CASES[2])
        for _step in range(10):
            _ = session.command(Command.INC)
        self.assertIs(session.report()[1][0], Level.WARN)

    def test_render_is_the_report_as_text(self):
        session = play(CASES[0])
        self.assertEqual(
            session.render(), "\n".join(text for _, text in session.report())
        )


class CommandLine(unittest.TestCase):
    def test_every_command_is_a_subcommand(self):
        parser = build_parser()
        for cmd in Command:
            with self.subTest(command=cmd):
                args = parser.parse_args([cmd.value])
                self.assertEqual(args.command, cmd.value)

    def test_every_command_documents_itself(self):
        for cmd in Command:
            with self.subTest(command=cmd):
                self.assertTrue(cmd.description)

    def test_daemon_accepts_calibration_flags(self):
        args = build_parser().parse_args(
            ["daemon", "--sensitivity=0.5", "--sigma-boat=0.002", "--chunk-offset=0"]
        )
        self.assertEqual(args.sensitivity, 0.5)
        self.assertEqual(args.sigma_boat, 0.002)
        self.assertEqual(args.chunk_offset, 0)

    def test_socket_is_overridable_everywhere(self):
        parser = build_parser()
        for argv in (["daemon", "--socket=/tmp/x"], ["reset", "--socket=/tmp/x"]):
            with self.subTest(argv=argv):
                self.assertEqual(parser.parse_args(argv).socket, "/tmp/x")

    def test_rejects_unknown_command(self):
        with self.assertRaises(SystemExit), mock.patch("sys.stderr"):
            _ = build_parser().parse_args(["wat"])

    def test_rejects_invalid_chunk_offset(self):
        with self.assertRaises(SystemExit), mock.patch("sys.stderr"):
            _ = build_parser().parse_args(["daemon", "--chunk-offset=4"])

    def test_requires_a_command(self):
        with self.assertRaises(SystemExit), mock.patch("sys.stderr"):
            _ = build_parser().parse_args([])


class StatusFile(unittest.TestCase):
    def test_writes_one_line_per_row(self):
        with tempfile.TemporaryDirectory() as directory:
            path = os.path.join(directory, "status")
            daemon.write_status(path, [(Level.HEAD, "chunk"), (Level.MAIN, "1, 2")])
            with open(path, encoding="utf-8") as status:
                self.assertEqual(status.read(), "head|chunk\nmain|1, 2\n")

    def test_a_multi_line_message_stays_one_row(self):
        """The overlay parses `<level>|<text>` line by line, so an embedded
        newline would silently drop the rest of the message."""
        with tempfile.TemporaryDirectory() as directory:
            path = os.path.join(directory, "status")
            daemon.write_status(path, [(Level.ERROR, "broke\nbecause\nreasons")])
            with open(path, encoding="utf-8") as status:
                self.assertEqual(status.read(), "error|broke because reasons\n")

    def test_replaces_the_previous_report_atomically(self):
        with tempfile.TemporaryDirectory() as directory:
            path = os.path.join(directory, "status")
            daemon.write_status(path, [(Level.DIM, "first")])
            daemon.write_status(path, [(Level.DIM, "second")])
            self.assertEqual(sorted(os.listdir(directory)), ["status"])
            with open(path, encoding="utf-8") as status:
                self.assertEqual(status.read(), "dim|second\n")


class FakeProcess:
    """A wl-paste that exits at once, the way it does without data-control."""

    def __init__(self, stderr: bytes):
        self.stdout: Iterator[bytes] = iter([])
        self.stderr: mock.Mock = mock.Mock(read=mock.Mock(return_value=stderr))

    def wait(self) -> int:
        return 1


class ClipboardSupervision(unittest.TestCase):
    def test_gives_up_instead_of_restarting_forever(self):
        """Retrying a compositor that will never support the protocol is what
        spammed waywall's log with one connection per second."""
        events: daemon.Events = queue.Queue()
        stderr = b"Failed to connect to a Wayland server\nNote: check the socket"
        with (
            mock.patch.object(
                daemon.subprocess, "Popen", return_value=FakeProcess(stderr)
            ) as popen,
            mock.patch.object(daemon.time, "sleep"),
            mock.patch("sys.stderr"),
        ):
            daemon.watch_clipboard(events)

        self.assertEqual(popen.call_count, daemon.MAX_RAPID_FAILURES)
        kind, payload = events.get_nowait()
        self.assertEqual(kind, "fatal")
        self.assertEqual(payload, "Failed to connect to a Wayland server")

    def test_a_watch_that_ran_for_a_while_is_retried(self):
        """Only immediate failures are fatal; a subscription that worked and
        then died is exactly what the restart loop is for."""
        events: daemon.Events = queue.Queue()
        clock = iter([0.0, 100.0] * daemon.MAX_RAPID_FAILURES + [0.0, 0.0] * 8)
        process = FakeProcess(b"")
        with (
            mock.patch.object(daemon.subprocess, "Popen", return_value=process),
            mock.patch.object(daemon.time, "sleep"),
            mock.patch.object(daemon.time, "monotonic", lambda: next(clock)),
            mock.patch("sys.stderr"),
        ):
            daemon.watch_clipboard(events)

        # It only reached the fatal path once the fake clock stopped advancing.
        self.assertEqual(events.get_nowait()[0], "fatal")
