{
  programs.claude-code = {
    enable = true;
  };

  preservation.preserveAt.state-dir.directories = [ ".claude" ];
}
