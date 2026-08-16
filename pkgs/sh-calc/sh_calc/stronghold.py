"""Where the stronghold is: rings, prior, measured rays and the posterior.

Vanilla places 128 strongholds in 8 rings.  Within a ring they sit at exactly
2*pi/count apart with one shared random rotation, at a uniform random radius
inside the ring, before being relocated to a valid biome up to SNAP chunks away.
That is the prior.  A throw contributes a ray; the posterior enumerates the
chunk lattice points near it, weights them by the ring density, conditions on
the measured angle, and finally conditions on the fact that an eye points at the
*nearest* stronghold.  The resulting percentage is a real posterior probability,
not a score.

The float32 rounding and Java's rounding mode are not numerical shortcuts:
Minecraft's yaw is a float32, so `f32` reconstructs a value that really did live
in 32 bits, and computing it in float64 would just be a precise representation
of a number the game never had.  Measured effect on the reference cases is
<=0.55% of sigma and no change to any predicted chunk -- kept because it is free
and matches the game, not because the answer hinges on it.
"""

from __future__ import annotations

import math
import struct
from dataclasses import dataclass
from enum import StrEnum

import numpy as np

# ==== CONFIGURATION AND JAVA SEMANTICS ==================================


@dataclass(frozen=True)
class Config:
    #: Minecraft raw mouse sensitivity.  This has to be exact -- it sets the
    #: angle quantum, and snapping a yaw onto the wrong grid throws the ray off
    #: by half an increment, which is several sigma.  Default carried over from
    #: Ninjabrain-Bot's calibration in ~/.java/.userPrefs/ninjabrainbot/prefs.xml
    #: and matches mouseSensitivity in standardsettings.json; pass --sensitivity
    #: if you change it in game.
    sensitivity: float = 0.02291165
    #: Residual angular error of a throw, in degrees.
    sigma_boat: float = 0.0007
    #: Per-user crosshair calibration, in degrees.
    crosshair_correction: float = 0.0
    #: Vertical resolution used for the subpixel inc/dec corrections.
    resolution_height: float = 16384.0
    #: Offset of the stronghold within its chunk: 8 before 1.19, 0 from 1.19 on.
    chunk_offset: int = 8


def f32(x: float) -> float:
    """Round to float32, matching Minecraft's yaw field."""
    return struct.unpack("<f", struct.pack("<f", x))[0]


def jround(x: float) -> int:
    """Java's Math.round: floor(x + 0.5).  Python's round() is banker's rounding."""
    return math.floor(x + 0.5)


def clamp180(a: float) -> float:
    """Fold an angle into [-180, 180]."""
    a = math.fmod(a, 360.0)  # math.fmod matches Java's %; Python's % does not
    if a < -180.0:
        return a + 360.0
    if a > 180.0:
        return a - 360.0
    return a


#: Java widens the float literals 0.6f/0.2f to double, which is not 0.6/0.2.
F0_6, F0_2 = f32(0.6), f32(0.2)


# ==== THE RINGS =========================================================


SNAP = 7  # biome-snapping radius, in chunks
DIST = 32  # ring spacing parameter
N_RINGS = 8
N_STRONGHOLDS = 128
MAX_CHUNK = int(DIST * ((4 + (N_RINGS - 1) * 6) + 1.25) + 2 * SNAP + 1)
SQRT2 = math.sqrt(2)


@dataclass(frozen=True)
class Ring:
    index: int
    count: int  #: strongholds in this ring, evenly spaced by 2*pi/count
    inner: float  #: radii in chunks, before biome snapping
    outer: float
    inner_snap: float  #: and after
    outer_snap: float


def _build_rings() -> list[Ring]:
    rings: list[Ring] = []
    count, placed = 1, 0
    for i in range(N_RINGS):
        count += 2 * count // (i + 1)
        count = min(count, N_STRONGHOLDS - placed)
        placed += count
        inner, outer = DIST * ((4 + i * 6) - 1.25), DIST * ((4 + i * 6) + 1.25)
        pad = (SNAP + 1.0) * SQRT2
        rings.append(Ring(i, count, inner, outer, inner - pad, outer + pad))
    return rings


RINGS = _build_rings()
_INNER_SNAP = np.array([r.inner_snap for r in RINGS])
_OUTER_SNAP = np.array([r.outer_snap for r in RINGS])


def ring_index(chunk_r: np.ndarray | float) -> np.ndarray:
    """Index of the ring a radius falls in (post-snapping bounds), or -1."""
    r = np.asarray(chunk_r, dtype=float)[..., None]
    inside = (r >= _INNER_SNAP) & (r <= _OUTER_SNAP)
    return np.where(inside.any(-1), inside.argmax(-1), -1)


def max_distance(x: float, z: float) -> float:
    """Furthest the *nearest* stronghold can possibly be from (x, z), in blocks."""
    r = math.hypot(x, z) / 16
    best = math.inf
    for ring in RINGS:
        c = math.cos(math.pi / ring.count)
        lo = ring.inner**2 + r * r - 2 * r * ring.inner * c
        hi = ring.outer**2 + r * r - 2 * r * ring.outer * c
        best = min(best, math.sqrt(max(lo, hi)))
    return (best + SQRT2 * (SNAP + 0.5)) * 16


# ==== THE PRIOR =========================================================


def _offset_weights() -> np.ndarray:
    """How often each chunk offset occurs, given the 4-block biome sample grid."""
    w = np.zeros(2 * SNAP + 1)
    for i in range(-26, 31):
        w[-(i >> 2) + SNAP] += 1  # >> matches Java's arithmetic shift
    return w


def _build_density() -> tuple[np.ndarray, np.ndarray]:
    n = MAX_CHUNK + 5
    density = np.zeros(n)
    for ring in RINGS:
        c0, c1 = int(ring.inner), int(ring.outer)
        r = np.arange(c0, c1 + 1)
        rho = ring.count / (2 * math.pi * (ring.outer - ring.inner) * r)
        rho[0] *= 0.5  # trapezoid ends
        rho[-1] *= 0.5
        density[c0 : c1 + 1] = rho

    # Radial blur induced by biome snapping: project each 2D snapping offset
    # onto a random radial direction and histogram the result.
    offsets = _offset_weights()
    kernel = np.zeros(math.ceil(SNAP * SQRT2) + 1)
    total = 0.0
    phis = 2 * math.pi * np.arange(200) / 200
    for k in range(-SNAP, SNAP + 1):
        for l in range(-SNAP, SNAP + 1):
            w = float(offsets[k + SNAP] * offsets[l + SNAP])
            dr = np.abs(np.floor(math.hypot(k, l) * np.sin(phis) + 0.5)).astype(int)
            kernel += np.bincount(
                dr, weights=np.full(dr.size, w), minlength=kernel.size
            )
            total += w * (2 * dr.size - int(np.count_nonzero(dr == 0)))
    kernel /= total

    symmetric = np.concatenate([kernel[:0:-1], kernel])
    density = np.convolve(density, symmetric, mode="same")
    cumulative = np.cumsum(density * np.arange(n) * 2 * math.pi)
    return density, cumulative


_DENSITY, _CUMULATIVE = _build_density()
_RADII = np.arange(len(_DENSITY), dtype=float)


def radial_density(cx: np.ndarray | float, cz: np.ndarray | float) -> np.ndarray:
    """Prior stronghold density at chunk coordinates (cx, cz)."""
    return np.interp(np.hypot(cx, cz), _RADII, _DENSITY)


def cumulative_polar(r: np.ndarray | float) -> np.ndarray:
    """Integral of the density over the disc of radius r (in chunks)."""
    return np.interp(r, _RADII, _CUMULATIVE)


# ==== MEASURED RAYS =====================================================


class Dimension(StrEnum):
    OVERWORLD = "overworld"
    NETHER = "the_nether"
    END = "the_end"

    @classmethod
    def from_namespaced_id(cls, world: str) -> Dimension | None:
        """`minecraft:the_nether` -> Dimension.NETHER."""
        return next((d for d in cls if world.endswith(d)), None)

    @property
    def scale(self) -> float:
        """Multiplier from this dimension's coordinates to overworld ones."""
        return 8.0 if self is Dimension.NETHER else 1.0


@dataclass(frozen=True)
class Position:
    x: float
    y: float
    z: float
    yaw: float
    pitch: float
    dimension: Dimension

    @property
    def scale(self) -> float:
        return self.dimension.scale

    @property
    def looking_below_horizon(self) -> bool:
        return self.pitch > 0


@dataclass(frozen=True)
class Throw:
    x: float  #: overworld coordinates
    z: float
    base: float  #: measured angle in degrees, before subpixel corrections
    pitch: float
    step: float  #: degrees per subpixel correction
    increments: int = 0

    @property
    def angle(self) -> float:
        return self.base + self.increments * self.step


def min_increment(cfg: Config) -> float:
    """Smallest yaw change a single mouse movement can produce, in degrees."""
    m = cfg.sensitivity * F0_6 + F0_2
    return m * m * m * 8.0 * 0.15


def correction_step(pitch: float, cfg: Config) -> float:
    """Angle change per subpixel (inc/dec) correction, in degrees.

    One pixel of the tall resolution, which is how you read the residual off the
    screen: aim as close as the increment grid allows, then count pixels.
    """
    rad = math.atan(2 * math.tan(math.radians(15)) / cfg.resolution_height)
    return math.degrees(rad) / math.cos(math.radians(pitch))


def make_throw(pos: Position, cfg: Config, anchor: float = 0.0) -> Throw:
    """The ray a throw defines, with the yaw snapped back onto its grid.

    `anchor` is where that grid sits, and 0 is the assumption this tool runs on.
    It is a parameter only because the reference measurements in tests/ were
    taken after measuring a real boat, which anchors the grid somewhere else.
    """
    inc = min_increment(cfg)
    alpha = f32(anchor + jround((pos.yaw - anchor) / inc) * inc)
    alpha += cfg.crosshair_correction
    alpha -= 0.000824 * math.sin(math.radians(alpha + 45))  # entity packet rounding
    return Throw(
        x=pos.x * pos.scale,
        z=pos.z * pos.scale,
        base=clamp180(alpha),
        pitch=pos.pitch,
        step=correction_step(pos.pitch, cfg),
    )


# ==== THE POSTERIOR =====================================================


#: Quadrature half-width for the angular integral in closest_stronghold_factor.
#: Converged: 7, 15, 31 and 63 give identical results.
K = 7
_KS = np.arange(-K, K + 1)

#: Half-width of the ray wedge, as a multiple of the throw's sigma...
WEDGE_SIGMAS = 30
#: ...but never narrower than this, in degrees.  See triangulate().
MIN_WEDGE_DEGREES = 0.05

#: Angular noise from F3+C rounding coordinates to two decimals, in degrees*blocks.
LATERAL_ERROR = 0.005 * SQRT2 * 180 / math.pi


def candidate_chunks(
    throw: Throw, tolerance: float, reach: float, cfg: Config
) -> tuple[np.ndarray, np.ndarray]:
    """Chunks whose centre lies within `tolerance` radians of the throw ray.

    A boat measurement has sigma ~ 0.001 degrees, which at 1500 blocks is 0.03
    blocks of lateral error against a 16-block chunk grid.  So this wedge is
    essentially a line, and it picks out only a handful of chunks -- all of the
    remaining ambiguity is *along* the ray, not across it.
    """
    o_x = (throw.x - cfg.chunk_offset) / 16
    o_z = (throw.z - cfg.chunk_offset) / 16
    phi = math.radians(throw.angle)
    dx, dz = -math.sin(phi), math.cos(phi)
    ux, uz = -math.sin(phi - tolerance), math.cos(phi - tolerance)
    vx, vz = -math.sin(phi + tolerance), math.cos(phi + tolerance)

    # March along whichever axis the ray advances fastest in, so each step
    # crosses exactly one lattice line.
    major_x = abs(dx) > abs(dz)
    if major_x:
        o_major, o_minor, d_major = o_x, o_z, dx
        slope_u, slope_v = uz / ux, vz / vx
    else:
        o_major, o_minor, d_major = o_z, o_x, dz
        slope_u, slope_v = ux / uz, vx / vz

    step = 1 if d_major > 0 else -1
    limit = reach * abs(d_major) + 1
    i = math.ceil(o_major) if step > 0 else math.floor(o_major)

    xs: list[int] = []
    zs: list[int] = []
    while abs(i - o_major) < limit:
        a = o_minor + slope_u * (i - o_major)
        b = o_minor + slope_v * (i - o_major)
        for j in range(math.ceil(min(a, b)), math.floor(max(a, b)) + 1):
            xs.append(i if major_x else j)
            zs.append(j if major_x else i)
        i += step
    return np.array(xs, dtype=np.int64), np.array(zs, dtype=np.int64)


def position_variance(dist2: np.ndarray | float, throw: Throw) -> np.ndarray | float:
    """Angular variance contributed by F3+C rounding the throw position."""

    def on_corner(v: float) -> bool:
        frac = v - math.floor(v)
        return abs(frac - 0.3) < 1e-6 or abs(frac - 0.7) < 1e-6

    if on_corner(throw.x) and on_corner(throw.z):
        return 0.0  # threw against a block corner: the position is exact
    return LATERAL_ERROR**2 / dist2 / 6  # variance of a uniform distribution


def angle_likelihood(
    cx: np.ndarray, cz: np.ndarray, throw: Throw, cfg: Config
) -> np.ndarray:
    """P(measured angle | stronghold in this chunk), up to a constant."""
    dx = cx * 16 + cfg.chunk_offset - throw.x
    dz = cz * 16 + cfg.chunk_offset - throw.z
    gamma = -np.degrees(np.arctan2(dx, dz))
    delta = np.abs((gamma - throw.angle) % 360.0)
    delta = np.minimum(delta, 360.0 - delta)
    variance = cfg.sigma_boat**2 + position_variance(dx * dx + dz * dz, throw)
    return np.exp(-delta * delta / (2 * variance))


def closest_stronghold_factor(
    cx: np.ndarray, cz: np.ndarray, throw: Throw, cfg: Config
) -> np.ndarray:
    """P(no other stronghold is closer) for each candidate chunk.

    An eye points at the *nearest* stronghold, so a candidate is only viable if
    every other stronghold is further away.  Strongholds in a ring sit at exactly
    2*pi/count apart, which makes this computable: for each other slot, integrate
    the probability that it lands inside the disc of radius d_i.
    """
    factor = np.zeros(len(cx))
    chunk_ring = ring_index(np.hypot(cx, cz))  # -1 -> impossible, stays 0
    r_p = math.hypot(throw.x, throw.z) / 16
    phi_p = -math.atan2(throw.x, throw.z)
    reach = max_distance(throw.x, throw.z) / 16

    for ri in np.unique(chunk_ring):
        if ri < 0:
            continue
        sel = chunk_ring == ri
        sx, sz = cx[sel], cz[sel]
        phi_chunk = -np.arctan2(sx, sz)[:, None]
        d_i = np.hypot(
            sx + (cfg.chunk_offset - throw.x) / 16,
            sz + (cfg.chunk_offset - throw.z) / 16,
        )[:, None]

        probability = np.ones(len(sx))
        for ring in RINGS:
            if r_p + reach < ring.inner or r_p - reach > ring.outer:
                continue
            same_ring = ring.index == ri
            if same_ring:
                # Neighbours in our own ring: their angle is pinned relative to
                # ours, blurred only by biome snapping.
                dphi = (2 / (2 * K + 1)) * 15 * SQRT2 / ring.inner
                u = _KS * dphi * ring.inner / (15 * SQRT2)
                pdf = (1 - u * u) ** 4.5
            else:
                # Other rings have an independent random rotation: uniform.
                dphi = (2 / (2 * K + 1)) * math.pi / ring.count
                pdf = np.ones(2 * K + 1)
            norm = pdf.sum() * dphi

            for slot in range(ring.count):
                if same_ring and slot == 0:
                    continue  # that slot is the candidate itself
                gamma = phi_p - (
                    phi_chunk + slot * 2 * math.pi / ring.count + _KS * dphi
                )
                sin_gamma = np.sin(gamma)
                sin_beta = r_p / d_i * sin_gamma
                ok = (np.abs(sin_beta) < 1) & (sin_gamma != 0)
                beta = np.arcsin(np.clip(sin_beta, -1, 1))
                safe = np.where(ok, sin_gamma, 1.0)
                # Where the slot's ring crosses the disc of radius d_i.
                r0 = np.clip(
                    d_i * np.sin(beta - gamma) / safe, ring.inner_snap, ring.outer_snap
                )
                r1 = np.clip(
                    d_i * np.sin(math.pi - gamma - beta) / safe,
                    ring.inner_snap,
                    ring.outer_snap,
                )
                inside = pdf * (cumulative_polar(r1) - cumulative_polar(r0))
                integral = np.where(ok, inside, 0.0).sum(1) * dphi / ring.count
                probability *= 1 - np.minimum(integral / norm, 1.0)

        factor[sel] = probability
    return factor


@dataclass(frozen=True)
class Prediction:
    cx: int
    cz: int
    certainty: float

    @property
    def x(self) -> int:
        return 16 * self.cx + 4

    @property
    def z(self) -> int:
        return 16 * self.cz + 4

    @property
    def ok(self) -> bool:
        return math.isfinite(self.certainty) and self.certainty > 0.0005

    def distance(self, pos: Position) -> int:
        return int(math.hypot(self.x - pos.x * pos.scale, self.z - pos.z * pos.scale))

    def angle_sigma(self, throw: Throw, cfg: Config) -> float:
        """The angular error the model expects of a throw landing on this chunk.

        For anything far away this is just sigma, but a stronghold a few blocks
        off is dominated by F3+C rounding the player's own position instead.
        """
        dx = 16 * self.cx + cfg.chunk_offset - throw.x
        dz = 16 * self.cz + cfg.chunk_offset - throw.z
        variance = position_variance(dx * dx + dz * dz, throw)
        return math.sqrt(cfg.sigma_boat**2 + float(variance))

    def angle_error(self, throw: Throw, cfg: Config) -> float:
        """How far the throw's ray misses this chunk by, in degrees.

        Ninjabrain-Bot's Chunk.getAngleError.  Against the winning chunk this is
        the measurement's residual, and the only check on it there is.
        """
        dx = 16 * self.cx + cfg.chunk_offset - throw.x
        dz = 16 * self.cz + cfg.chunk_offset - throw.z
        gamma = -math.degrees(math.atan2(dx, dz))
        delta = math.fmod(throw.angle - gamma, 360.0)
        if delta < -180.0:
            return delta + 360.0
        if delta > 180.0:
            return delta - 360.0
        return delta


def triangulate(throws: list[Throw], cfg: Config, top: int = 5) -> list[Prediction]:
    """Posterior over stronghold chunks, most likely first."""
    if not throws:
        return []
    first = throws[0]
    reach = max_distance(first.x, first.z) / 16
    if reach * cfg.sigma_boat > 1000:
        return []  # too many candidates to be meaningful

    # WEDGE_SIGMAS covers the angular measurement, but for a stronghold only a
    # few blocks away the dominant error is F3+C's coordinate rounding, not
    # sigma.  Hence the absolute floor: without it a very close stronghold can
    # fall outside the wedge, and the posterior then renormalises onto the wrong
    # chunk and reports high confidence.  Widening past this changes nothing.
    tolerance = math.radians(
        min(1.0, max(WEDGE_SIGMAS * cfg.sigma_boat, MIN_WEDGE_DEGREES))
    )
    cx, cz = candidate_chunks(first, tolerance, reach + 2, cfg)
    if len(cx) == 0:
        return []

    # Prior: ring density integrated over each chunk on a 4x4 grid, which is
    # where the quadrature converges.
    steps = np.linspace(-0.5, 0.5, 4)
    weight = np.mean(
        [radial_density(cx + a, cz + b) for a in steps for b in steps], axis=0
    )
    dx, dz = cx - first.x / 16, cz - first.z / 16
    weight[dx * dx + dz * dz > reach * reach] = 0

    for throw in throws:
        weight = weight * angle_likelihood(cx, cz, throw, cfg)
        if weight.sum() <= 0:
            return []
        weight /= weight.sum()

    weight *= closest_stronghold_factor(cx, cz, first, cfg)
    if weight.sum() <= 0:
        return []
    weight /= weight.sum()

    best = np.argsort(weight)[::-1][:top]
    return [Prediction(int(cx[i]), int(cz[i]), float(weight[i])) for i in best]
