{ inputs, pkgs, ... }:
{
  imports = [ inputs.spicetify-nix.hjemModules.default ];

  programs.spicetify =
    let
      spicePkgs = inputs.spicetify-nix.legacyPackages.${pkgs.stdenv.hostPlatform.system};
    in
    {
      enable = true;
      enabledExtensions = with spicePkgs.extensions; [
        fullAppDisplay
        history
        hidePodcasts
        playNext
        volumePercentage
        showQueueDuration
        copyToClipboard
      ];
    };

  preservation.preserveAt.state-dir.directories = [ ".config/spotify" ];
}
