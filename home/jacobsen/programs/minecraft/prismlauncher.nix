{
  # TODO: setup prismlauncher settings
  programs.prismlauncher.enable = true;

  preservation.preserveAt.state-dir.directories = [
    ".local/share/PrismLauncher"
  ];
}
