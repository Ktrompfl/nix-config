{
  # steam is installed in system settings for correct hardware configuration
  preservation.preserveAt.state-dir.directories = [
    ".steam"
    ".local/share/Steam"

    # individual game save directories
    ".local/share/Celeste"
    ".local/share/Larian Studios"
    ".local/share/Terraria"
  ];
}
