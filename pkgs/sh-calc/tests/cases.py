"""Reference measurements shared by the tests.

Real measurements with known stronghold chunks, taken from Ninjabrain-Bot's
src/test/java/ninjabrainbot/integrationtests/, BoatCalculatedTravelTests.java
(one boat throw) and CalculatedTravelTests.java (two throws).
"""

from __future__ import annotations

from dataclasses import dataclass

from sh_calc.daemon import Command, Session, State, parse_f3c
from sh_calc.stronghold import (
    Config,
    Position,
    Prediction,
    Throw,
    jround,
    make_throw,
    triangulate,
)


@dataclass(frozen=True)
class Case:
    config: Config
    boat_f3c: str
    eye_f3c: str
    corrections: int
    expected: tuple[int, int]


BOAT = Config(sensitivity=0.065292805, sigma_boat=0.001)
#: The stronghold in this one is 17 blocks away, which makes F3+C's coordinate
#: rounding the dominant angular error rather than sigma.
DOOGILE = Config(sensitivity=0.00467673, sigma_boat=0.0007)

#: One throw, after measuring a real boat.  The boat reading is not a throw --
#: it only says where the yaw grid is anchored, which for these is not 0.
CASES = [
    Case(
        BOAT,
        "/execute in minecraft:overworld run tp @s 1274.04 92.55 1064.56 -78.75 32.82",
        "/execute in minecraft:overworld run tp @s 1275.31 93.00 1064.81 -146.11 -32.13",
        7,
        (133, -14),
    ),
    Case(
        BOAT,
        "/execute in minecraft:overworld run tp @s -1380.20 80.00 1138.90 -143.16 28.53",
        "/execute in minecraft:overworld run tp @s -1376.84 80.00 1132.87 -60.46 -31.73",
        5,
        (-43, 95),
    ),
    Case(
        BOAT,
        "/execute in minecraft:overworld run tp @s 3430.24 62.07 -4805.87 104.06 74.04",
        "/execute in minecraft:overworld run tp @s 3430.24 63.09 -4805.87 93.92 -31.81",
        23,
        (111, -308),
    ),
    Case(
        DOOGILE,
        "/execute in minecraft:overworld run tp @s -1306.60 62.07 587.31 -2.67 74.04",
        "/execute in minecraft:overworld run tp @s -1306.60 62.07 587.31 -55.68 -31.81",
        0,
        (-81, 37),
    ),
]

#: Ninjabrain-Bot's withProSettings(): two throws, no boat, measured carefully.
#: The grid anchor is unknown for these, so they exercise the posterior rather
#: than the yaw reconstruction.
PRO = Config(sigma_boat=0.005)

#: Two throws from CalculatedTravelTests.java, with the answer.
PLAIN_CASES = [
    (
        "/execute in minecraft:overworld run tp @s 199.50 71.00 -63.50 60.23 -31.39",
        "/execute in minecraft:overworld run tp @s 196.49 69.00 -69.35 59.99 -31.52",
        (-78, 47),
    ),
    (
        "/execute in minecraft:overworld run tp @s 809.90 69.00 -2091.99 91.72 -31.27",
        "/execute in minecraft:overworld run tp @s 812.25 63.09 -2100.82 91.48 -31.60",
        (-74, -135),
    ),
    (
        "/execute in minecraft:overworld run tp @s 4798.40 63.00 -307.89 174.14 -31.39",
        "/execute in minecraft:overworld run tp @s 4786.13 64.00 -309.25 174.56 -31.60",
        (289, -121),
    ),
]

#: The player walked between the two throws, so the rays do not meet anywhere
#: plausible and Ninjabrain-Bot reports a failure.
MOVING_CASES = [
    (
        "/execute in minecraft:overworld run tp @s -14.76 65.00 46.01 193.84 -31.23",
        "/execute in minecraft:overworld run tp @s -24.67 64.00 43.06 194.73 -31.48",
    ),
    (
        "/execute in minecraft:overworld run tp @s 2006.33 67.00 3960.43 370.84 -31.19",
        "/execute in minecraft:overworld run tp @s 2016.72 68.00 3970.79 372.41 -31.60",
    ),
    (
        "/execute in minecraft:overworld run tp @s -1917.30 65.00 -6.70 12.77 -31.31",
        "/execute in minecraft:overworld run tp @s -1907.40 65.00 -3.65 14.67 -31.64",
    ),
]


def parse(text: str) -> Position:
    pos = parse_f3c(text)
    assert pos is not None, f"unparseable F3+C: {text}"
    return pos


def boat_anchor(yaw: float) -> float:
    """Where a boat at this yaw anchors the grid.

    A boat's rotation is quantised to 360/256 degrees, or 360/2560 when
    negative, so the two decimals F3+C prints round-trip through it exactly.
    Ninjabrain-Bot's SetBoatAngleAction; it lives here rather than in the
    package because the tool itself never measures a boat.
    """
    step = 1.40625 if yaw >= 0 else 0.140625
    return jround(yaw / step) * step


def fresh() -> Session:
    return Session(BOAT)


def session_of(cfg: Config, throws: tuple[Throw, ...], player: Position) -> Session:
    """A session holding throws the daemon could not have built itself."""
    session = Session(cfg)
    session.state = State(throws=throws)
    session.player = player
    return session


def play(case: Case) -> Session:
    """Drive a session as the daemon would, but anchored on the measured boat.

    The daemon always anchors the yaw grid at 0; these measurements were taken
    against a real boat, which anchors it at the boat's own yaw instead.  That
    is the only difference, so the throw is built with that anchor and the
    corrections are then applied through the session as usual.
    """
    eye = parse(case.eye_f3c)
    throw = make_throw(eye, case.config, anchor=boat_anchor(parse(case.boat_f3c).yaw))
    session = session_of(case.config, (throw,), eye)
    for _step in range(abs(case.corrections)):
        _ = session.command(Command.INC if case.corrections > 0 else Command.DEC)
    return session


def predict(case: Case) -> Prediction:
    return triangulate(list(play(case).state.throws), case.config)[0]


def predict_plain(f3c1: str, f3c2: str, cfg: Config = PRO) -> list[Prediction]:
    """Two throws with no grid to snap to.

    Anchoring each throw on its own yaw makes the snapping a no-op, which is
    what Ninjabrain-Bot does for a throw with no boat -- these measurements come
    from a run without one, so there is no grid to put them back on.
    """
    throws = tuple(
        make_throw(pos, cfg, anchor=pos.yaw)
        for pos in (parse(f3c1), parse(f3c2))
    )
    return triangulate(list(throws), cfg)
