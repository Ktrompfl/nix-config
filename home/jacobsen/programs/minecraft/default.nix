{
  config,
  pkgs,
  ...
}:
{
  home.packages = with pkgs; [
    ninjabrainbot # stronghold calculator
  ];

  programs.prismlauncher.enable = true;

  home.file = {
    ".config/waywall" = {
      enable = true;
      # recursive = true;
      # -- FIXME: once the waywall config is stable put it into store and thereby make it immutable
      source = config.lib.file.mkOutOfStoreSymlink "/persist/nixos/home/jacobsen/programs/minecraft/waywall";
    };
  };

  preservation.preserveAt.state-dir.directories = [
    ".java/.userPrefs/ninjabrainbot" # preferences
    # ".local/share/PrismLauncher"
  ];
}
