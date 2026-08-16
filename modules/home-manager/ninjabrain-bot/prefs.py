"""Compile a declarative Ninjabrain Bot configuration into its prefs.xml.

The bot keeps its settings in java.util.prefs: flat key/value entries, all
plain scalars except the themes, which are packed into one encoded string --
hence the theme serialiser here, also reachable with --theme-only.
"""

from __future__ import annotations

import argparse
import json
import sys
from xml.sax.saxutils import quoteattr

type Entry = bool | float | int | str

# ThemeSerializer.java packs a value six bits at a time into printable
# characters starting at '0', most significant group first.
FILTER_6BIT = 0b111111
SHIFT_SIZE = 6

#: CustomTheme.java's colours, each with the uid it is stored under and the
#: base16 role filling it.  Uids must stay unique; there is no "g".
THEME_COLORS: list[tuple[str, str]] = [
    ("a", "base00"),  # title bar
    ("b", "base01"),  # header background
    ("c", "base01"),  # result background
    ("d", "base01"),  # throws background
    ("e", "base00"),  # dividers
    ("f", "base00"),  # header dividers
    ("h", "base05"),  # text
    ("n", "base05"),  # title text
    ("k", "base05"),  # throws text
    ("i", "base05"),  # divine text
    ("j", "base04"),  # version text
    ("o", "base04"),  # header text
    ("l", "base0B"),  # subpixel +
    ("m", "base08"),  # subpixel -
    ("r", "base0B"),  # certainty 100%
    ("q", "base0E"),  # certainty 50%
    ("p", "base08"),  # certainty 0%
]


def serialize_int(value: int, bits: int) -> str:
    """Pack `value` into `bits` bits worth of six bit characters."""
    result = ""
    while bits > 0:
        result = chr((value & FILTER_6BIT) + ord("0")) + result
        value >>= SHIFT_SIZE
        bits -= SHIFT_SIZE
    return result


def serialize_color(hex_color: str) -> str:
    """A colour as the four characters the bot stores it as."""
    return serialize_int(int(hex_color.lstrip("#"), 16), 24)


def theme_string(palette: dict[str, str]) -> str:
    """Encode a base16 palette as one of the bot's custom themes."""
    missing = sorted({role for _, role in THEME_COLORS} - set(palette))
    if missing:
        sys.exit(f"prefs.py: palette is missing {', '.join(missing)}")
    return "".join(uid + serialize_color(palette[role]) for uid, role in THEME_COLORS)


def entry_value(value: Entry) -> str:
    """Render a value the way java.util.prefs will read it back."""
    # bool first: also an int in Python, but Java wants true/false.
    if isinstance(value, bool):
        return "true" if value else "false"
    return str(value)


def render(settings: dict[str, Entry]) -> str:
    """The prefs.xml document for a set of already resolved entries."""
    lines = [
        '<?xml version="1.0" encoding="UTF-8" standalone="no"?>',
        '<!DOCTYPE map SYSTEM "http://java.sun.com/dtd/preferences.dtd">',
        '<map MAP_XML_VERSION="1.0">',
    ]
    # Sorted so the file only changes when the configuration does.
    for key in sorted(settings):
        value = quoteattr(entry_value(settings[key]))
        lines.append(f"  <entry key={quoteattr(key)} value={value}/>")
    lines.append("</map>")
    return "\n".join(lines) + "\n"


def add_custom_theme(settings: dict[str, Entry], name: str, encoded: str) -> None:
    """Put a theme first in the bot's "."-separated custom theme lists.

    Theme.java gives the first custom theme the uid -1, so first is what makes
    `theme = -1` select it.
    """
    if "." in name:
        sys.exit(f"prefs.py: theme name {name!r} may not contain a '.'")
    for key, value in (("custom_themes", encoded), ("custom_themes_names", name)):
        existing = settings.get(key, "")
        settings[key] = f"{value}.{existing}" if existing else value


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--settings", type=argparse.FileType(), help="JSON of prefs entries")
    parser.add_argument("--colors", type=argparse.FileType(), help="JSON of a base16 palette")
    parser.add_argument("--theme-name", default="Stylix", help="name for the palette's theme")
    parser.add_argument("--theme-only", action="store_true", help="print the theme string only")
    args = parser.parse_args()

    palette: dict[str, str] | None = json.load(args.colors) if args.colors else None

    if args.theme_only:
        if palette is None:
            sys.exit("prefs.py: --theme-only needs --colors")
        print(theme_string(palette))
        return

    settings: dict[str, Entry] = json.load(args.settings) if args.settings else {}
    if palette is not None:
        add_custom_theme(settings, args.theme_name, theme_string(palette))

    sys.stdout.write(render(settings))


if __name__ == "__main__":
    main()
