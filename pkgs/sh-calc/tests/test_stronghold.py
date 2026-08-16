"""The model: Java semantics, the rings, measured rays and the posterior.

The reference cases are real measurements from Ninjabrain-Bot's integration
tests, so agreeing with them is the only end-to-end check that matters.
"""

from __future__ import annotations

import itertools
import math
import time
import unittest
from unittest import mock

import numpy as np

from sh_calc import stronghold
from sh_calc.stronghold import (
    MAX_CHUNK,
    N_STRONGHOLDS,
    RINGS,
    Config,
    Position,
    correction_step,
    f32,
    make_throw,
    min_increment,
)
from tests.cases import (
    BOAT,
    CASES,
    MOVING_CASES,
    PLAIN_CASES,
    parse,
    play,
    predict,
    predict_plain,
)


class JavaSemantics(unittest.TestCase):
    def test_jround_is_floor_of_x_plus_half(self):
        for value, expected in [(0.5, 1), (1.5, 2), (2.5, 3), (-0.5, 0), (-1.5, -1)]:
            with self.subTest(value=value):
                self.assertEqual(stronghold.jround(value), expected)

    def test_jround_differs_from_python_round(self):
        self.assertNotEqual(stronghold.jround(0.5), round(0.5))
        self.assertNotEqual(stronghold.jround(2.5), round(2.5))

    def test_min_increment_uses_widened_float_literals(self):
        """Java widens the literals 0.6f/0.2f to double, which is not 0.6/0.2."""
        cfg = Config(sensitivity=0.065292805)
        naive = (cfg.sensitivity * 0.6 + 0.2) ** 3 * 8.0 * 0.15
        self.assertNotEqual(min_increment(cfg), naive)
        self.assertAlmostEqual(min_increment(cfg), naive, places=8)

    def test_clamp180(self):
        for value, expected in [
            (0.0, 0.0),
            (180.0, 180.0),
            (-180.0, -180.0),
            (190.0, -170.0),
            (-190.0, 170.0),
            (360.0, 0.0),
        ]:
            with self.subTest(value=value):
                self.assertAlmostEqual(stronghold.clamp180(value), expected)


class RingGeometry(unittest.TestCase):
    def test_stronghold_counts_match_vanilla(self):
        self.assertEqual([r.count for r in RINGS], [3, 6, 10, 15, 21, 28, 36, 9])

    def test_all_strongholds_are_placed(self):
        self.assertEqual(sum(r.count for r in RINGS), N_STRONGHOLDS)

    def test_rings_are_ordered_and_do_not_overlap(self):
        for inner, outer in itertools.pairwise(RINGS):
            self.assertLess(inner.inner, inner.outer)
            self.assertLess(inner.outer_snap, outer.inner_snap)

    def test_ring_index_identifies_membership(self):
        self.assertEqual(stronghold.ring_index(RINGS[0].inner), 0)
        self.assertEqual(stronghold.ring_index(RINGS[1].inner), 1)

    def test_ring_index_returns_minus_one_between_rings(self):
        gap = (RINGS[0].outer_snap + RINGS[1].inner_snap) / 2
        self.assertEqual(stronghold.ring_index(gap), -1)
        self.assertEqual(stronghold.ring_index(10 * MAX_CHUNK), -1)

    def test_max_distance_is_finite_and_positive(self):
        for x, z in [(0.0, 0.0), (1274.0, 1064.0), (-30000.0, 20000.0)]:
            with self.subTest(x=x, z=z):
                self.assertTrue(0 < stronghold.max_distance(x, z) < math.inf)


class YawGrid(unittest.TestCase):
    """The yaw only moves in whole increments from wherever it was last set, so
    the two decimals F3+C prints are enough to put it back on that grid."""

    POS: Position = parse(
        "/execute in minecraft:overworld run tp @s 0.0 64.0 0.0 -60.46 -31.0"
    )

    def test_snapping_recovers_a_grid_value_f3c_had_rounded_away(self):
        """F3+C throws away up to 0.005 degrees; snapping puts the yaw back to
        within the packet-rounding correction, an order of magnitude less."""
        cfg = Config()
        inc = min_increment(cfg)
        for n in (-3000, -17, 0, 1, 1234):
            with self.subTest(n=n):
                exact = f32(n * inc)
                pos = parse(
                    "/execute in minecraft:overworld run tp @s 0.0 64.0 0.0 "
                    f"{round(exact, 2)} -31.0"
                )
                self.assertLess(abs(make_throw(pos, cfg).base - exact), 0.001)

    def test_f3c_rounding_stays_inside_half_an_increment(self):
        """Why this works at all: F3+C can be off by 0.005 degrees, which has to
        stay under half an increment or the snap picks the wrong grid point."""
        self.assertGreater(min_increment(Config()) / 2, 0.005)

    def test_grid_values_are_exact_in_float32(self):
        """Why the grid needs no float32 rounding of its own: a boat's steps are
        45/32 and 9/64, so n*step is exact across the whole valid range."""
        for n in range(257):
            self.assertEqual(f32(n * 1.40625), n * 1.40625)
        for n in range(-2560, 1):
            self.assertEqual(f32(n * 0.140625), n * 0.140625)

    def test_min_increment_matches_god_sensitivity(self):
        god = Config(sensitivity=0.012727597)
        self.assertAlmostEqual(min_increment(god), 0.010742187555262477, places=12)

    def test_anchoring_a_throw_on_its_own_yaw_does_not_snap_it(self):
        unsnapped = make_throw(self.POS, BOAT, anchor=self.POS.yaw)
        self.assertLess(abs(unsnapped.base - self.POS.yaw), 0.001)

    def test_snapping_never_moves_an_angle_more_than_half_an_increment(self):
        moved = (
            make_throw(self.POS, BOAT).base
            - make_throw(self.POS, BOAT, anchor=self.POS.yaw).base
        )
        self.assertLessEqual(abs(moved), min_increment(BOAT) / 2)


class SubpixelCorrections(unittest.TestCase):
    def test_correction_step_grows_away_from_the_horizon(self):
        cfg = Config()
        self.assertLess(
            abs(correction_step(0.0, cfg)), abs(correction_step(-45.0, cfg))
        )

    def test_corrections_accumulate_linearly(self):
        session = play(CASES[0])
        throw = session.state.throws[-1]
        self.assertEqual(throw.increments, CASES[0].corrections)
        self.assertAlmostEqual(
            throw.angle, throw.base + throw.increments * throw.step, places=12
        )


class ReferenceCases(unittest.TestCase):
    def test_predicts_the_known_stronghold_chunk(self):
        for i, case in enumerate(CASES, 1):
            with self.subTest(case=i):
                best = predict(case)
                self.assertEqual((best.cx, best.cz), case.expected)

    def test_prediction_is_confident(self):
        for i, case in enumerate(CASES, 1):
            with self.subTest(case=i):
                best = predict(case)
                self.assertTrue(best.ok)
                self.assertGreater(best.certainty, 0.90)

    def test_overworld_and_nether_coordinates_agree(self):
        best = predict(CASES[0])
        self.assertEqual((best.x, best.z), (16 * best.cx + 4, 16 * best.cz + 4))
        self.assertEqual((best.x // 8, best.z // 8), (266, -28))


class PlainThrowReferenceCases(unittest.TestCase):
    """Two throws without a boat, which is a different regime: the wedge is
    wide, hundreds of chunks are candidates, and it is the intersection of the
    two rays rather than one ray's precision that pins the answer down."""

    def test_predicts_the_known_stronghold_chunk(self):
        for i, (f1, f2, expected) in enumerate(PLAIN_CASES, 1):
            with self.subTest(case=i):
                best = predict_plain(f1, f2)[0]
                self.assertEqual((best.cx, best.cz), expected)
                self.assertTrue(best.ok)

    def test_throws_taken_while_moving_do_not_produce_a_prediction(self):
        for i, (f1, f2) in enumerate(MOVING_CASES, 1):
            with self.subTest(case=i):
                predictions = predict_plain(f1, f2)
                self.assertFalse(predictions and predictions[0].ok)


class RayWedgeTruncation(unittest.TestCase):
    """The candidate wedge is sized from sigma, which is not always the dominant
    error.  Truncating it is silently dangerous: dropping the true chunk does not
    lower the reported confidence, it raises it, because the posterior
    renormalises over whatever survives."""

    def test_close_stronghold_error_is_dominated_by_coordinate_rounding(self):
        case = CASES[3]
        throw = play(case).state.throws[0]
        distance = math.hypot(
            16 * case.expected[0] + 8 - throw.x, 16 * case.expected[1] + 8 - throw.z
        )
        self.assertLess(distance, 25, "expected the near-stronghold case")
        rounding_sigma = math.sqrt(stronghold.position_variance(distance**2, throw))
        self.assertGreater(rounding_sigma, 10 * case.config.sigma_boat)

    def test_a_wedge_sized_only_from_sigma_truncates_that_case(self):
        case = CASES[3]
        with mock.patch.multiple(stronghold, WEDGE_SIGMAS=10, MIN_WEDGE_DEGREES=0.0):
            best = predict(case)
        self.assertNotEqual((best.cx, best.cz), case.expected)
        self.assertGreater(best.certainty, 0.9, "truncation hides itself as confidence")

    def test_result_is_stable_against_a_wider_wedge(self):
        for case in CASES:
            reference = predict(case)
            for floor in (0.1, 0.2, 0.4):
                with self.subTest(case=case.expected, floor=floor):
                    with mock.patch.object(stronghold, "MIN_WEDGE_DEGREES", floor):
                        best = predict(case)
                    self.assertEqual((best.cx, best.cz), (reference.cx, reference.cz))
                    self.assertAlmostEqual(best.certainty, reference.certainty, places=4)

    def test_quadrature_is_converged(self):
        reference = [predict(case).certainty for case in CASES]
        for k in (15, 31):
            with self.subTest(K=k):
                with mock.patch.multiple(stronghold, K=k, _KS=np.arange(-k, k + 1)):
                    widened = [predict(case).certainty for case in CASES]
                for got, want in zip(widened, reference):
                    self.assertAlmostEqual(got, want, places=4)


class Performance(unittest.TestCase):
    def test_triangulation_is_fast(self):
        case = CASES[0]
        throws = list(play(case).state.throws)
        start = time.perf_counter()
        for _call in range(20):
            _ = stronghold.triangulate(throws, case.config)
        per_call = (time.perf_counter() - start) / 20
        # Generously above the ~0.5 ms observed, to catch complexity regressions
        # rather than machine noise.
        self.assertLess(per_call, 0.05)
