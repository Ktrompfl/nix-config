{
  # Steam itself is installed in the system configuration for correct hardware
  # support; only its state belongs to the user. The library and the games'
  # saves follow the xdg data home, but this one is hard-coded.
  preservation.preserveAt.state-dir.directories = [ ".steam" ];
}
