# ninjabrain-box

Ninjabrain Bot on an X server of its own, with its panel drawn as a Wayland
layer-shell overlay that can sit above a fullscreen game.

`ninjabrain-box` starts both. With arguments it sends actions to the running
one: `show`, `hide`, `toggle`, `show-throws`, `hide-throws`, `toggle-throws`,
`reload`, `quit`, and the bot's own `reset`, `undo`, `redo`, `increment`,
`decrement`, `boat`, `lock`, `alt-std`, `mod-360`, `aa-mode`, `minimize`.
`--list` prints them.

> [!IMPORTANT]
> This runs the official Ninjabrain Bot build, normally, inside an X server,
> and uses its API to display the same information the bot would display
> itself.

## Configuration

`$XDG_CONFIG_HOME/ninjabrain-box/config.toml`, reloaded with `ninjabrain-box
reload`. Every key may be left out; these are the defaults.

```toml
[bot]
api = "127.0.0.1:52533"

# The bot's own defaults. Only what changes the arithmetic is configurable.
[bot.settings]
mc-version = "pre-1.19"                 # or "1.19+"
angle-adjustment = "subpixel"           # or "tall", "custom"
boat-type = "gray"                      # or "blue", "green"
all-advancements = false
all-advancements-toggle = "automatic"   # or "hotkey"
all-advancements-1-20-plus = false
use-precise-angle = false
use-alt-std = false
use-advanced-statistics = true
sensitivity = 0.012727597
sensitivity-manual = 0.4341732
sigma = 0.1
sigma-alt = 0.1
sigma-manual = 0.03
sigma-boat = 0.001
boat-error = 0.03
resolution-height = 16384.0
custom-adjustment = 0.01
crosshair-correction = 0.0
auto-reset = false
auto-reset-on-instance-change = false
save-state = true

[window]
# output = "DP-2"                       # unset lets the compositor choose
anchor = ["top", "right"]               # one edge only centres along it
margin = [8, 8, 8, 8]                   # top, right, bottom, left
coordinates = "chunk"                   # or "block"
angle-correction = "increments"         # or "degrees"
predictions = 4
color-negative-coordinates = true
# font = "/path/to/Mono.ttf"            # must be monospaced
font-size = 15.0
padding = 6
opacity = 0.85

[behavior]
start-hidden = false
start-with-throws = false
only-when-focused = []                  # app id or title substrings

# base16: 00 background, 01 headers, 02 alternating rows, 03 muted, 05 text,
# 08-0B the certainty gradient and correction signs, 0C nether, 0D distance,
# 0E angles.
[palette]
base00 = "#161616"
base01 = "#1f1f1f"
base02 = "#2a2a2a"
base03 = "#6c6c6c"
base04 = "#8f8f8f"
base05 = "#d8d8d8"
base06 = "#e8e8e8"
base07 = "#f8f8f8"
base08 = "#e06464"
base09 = "#e09a5a"
base0A = "#e0d064"
base0B = "#7cc96e"
base0C = "#6ec9c0"
base0D = "#6e9cc9"
base0E = "#b48ec9"
base0F = "#a16a4b"
```

Changing `[bot.settings]` needs a restart; everything else applies on reload.

## Requires

This requires the following Wayland protocols:

- `zwlr_layer_shell_v1`
- `ext_data_control_v1` or `zwlr_data_control_v1`
- `zwlr_foreign_toplevel_manager_v1`, for `only-when-focused`
