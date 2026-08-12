{ jayLib, ... }:
{
  # Compositor-wide behaviour that belongs to no particular device, window, or
  # key.
  window-management-key = "Super_L"; # logo uses different symbol names

  focus-follows-mouse = true;
  unstable-mouse-follows-focus = true;
  fallback-output-mode = "focus"; # more useful with mouse-follows-focus
  workspace-display-order = "sorted";
  middle-click-paste = false;
  show-titles = true;

  # Not available in the version of the jay-config crate that the shared
  # library configuration is pinned to, so only the toml side has it.
  split-reuses-container = true;

  idle = jayLib.idle;
  on-idle = "$suspend";
}
