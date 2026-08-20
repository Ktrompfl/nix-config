{ inputs, ... }:
{
  # This one brings our custom packages from the 'pkgs' directory
  additions =
    final: _prev:
    import ../pkgs {
      inherit (final) pkgs;
      inherit inputs;
    };

  # This one contains whatever you want to overlay
  # You can change versions, add patches, set compilation flags, anything really.
  # https://nixos.wiki/wiki/Overlays
  modifications = final: prev: {
    # Wraps the bot in a java agent that exposes its actions on a socket, so
    # that a compositor can drive it. Drop-in: same name, same binary, and on
    # anything but Wayland the bot itself is untouched.
    ninjabrain-bot = final.callPackage ../pkgs/ninjabrain-bot {
      ninjabrain-bot = prev.ninjabrain-bot;
    };

    # moonlight-qt 6.1.0 predates upstream's ffmpeg 7.1 API migration and no longer builds
    # against current ffmpeg. Follow master until 6.2.0 releases, as in
    # https://github.com/NixOS/nixpkgs/pull/552544
    # moonlight-qt = prev.moonlight-qt.overrideAttrs (oldAttrs: {
    #   version = "6.1.0-unstable-2026-08-06";

    #   src = final.fetchFromGitHub {
    #     owner = "moonlight-stream";
    #     repo = "moonlight-qt";
    #     rev = "2e13ed9977bc31c73caf8428f08f58d793313ece";
    #     hash = "sha256-kCm/YoFGcXhF/Abi5lRV5F7H1AbKJchdDOlfBVR0tRA=";
    #     fetchSubmodules = true;
    #   };

    #   # the Xcode < 14 fix is already in master
    #   patches = [ ];

    #   # would point at a tag that doesn't exist
    #   meta = removeAttrs oldAttrs.meta [ "changelog" ];
    # });
  };
}
