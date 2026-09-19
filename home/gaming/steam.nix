{
  # steam itself is installed in the system configuration for correct hardware
  # support; only its state belongs to the user
  preservation.preserveAt.state-dir.directories = [
    ".steam"
    ".local/share/Steam"

    # individual game save directories
    ".local/share/Celeste"
    ".local/share/Larian Studios"
    ".local/share/Terraria"
  ];
}
