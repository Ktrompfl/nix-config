{ inputs, pkgs, ... }:
inputs.jail.lib.extend {
  inherit pkgs;

  # Bind the whole store instead of each app's closure: jail.nix computes that
  # at every launch, which takes most of a second, and the store is
  # world-readable anyway. The stub turns jail.nix's closure walk into a no-op;
  # the bogus `runtime-deep-ro-bind` makes jail.nix define it first, so the
  # stub can replace it.
  basePermissions =
    c: with c; [
      base
      fake-passwd

      (unsafe-add-raw-args "--ro-bind /nix/store /nix/store")
      (runtime-deep-ro-bind "/nonexistent-emits-the-helpers")
      (add-runtime "bindNixStoreClosure() { :; }")
    ];
}
