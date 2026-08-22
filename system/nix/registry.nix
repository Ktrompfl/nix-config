{
  config,
  inputs,
  lib,
  ...
}:
let
  flakeInputs = lib.filterAttrs (_: v: lib.isType "flake" v) inputs;
in
{
  # Every input of flake.nix under its own name, so that `nix run nixpkgs#...`
  # and friends reuse the revision the system was built from
  # instead of fetching and evaluating a fresh nixpkgs.
  nix.registry = lib.mapAttrs (_: v: { flake = v; }) flakeInputs;

  # The same pins as `NIX_PATH`, for anything that still reads channels.
  nix.nixPath = lib.mapAttrsToList (key: _: "${key}=flake:${key}") config.nix.registry;

  nix.channel.enable = false;

  # `nix.registry` above is the *system* registry, which nix reads from
  # /etc/nix/registry.json on its own. This is the *global* one, which it
  # would otherwise take from the registry vendored into lix; emptying it
  # means an unpinned flake reference fails rather than silently resolving to
  # something this flake never locked.
  nix.settings.flake-registry = "";
}
