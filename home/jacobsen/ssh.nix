{
  # persist ssh keys / known hosts
  preservation.preserveAt.data-dir.directories = [
    {
      directory = ".ssh";
      mode = "0700";
    }
  ];
}
