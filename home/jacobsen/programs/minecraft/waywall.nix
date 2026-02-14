{
  config,
  ...
}:
{
  # TODO: create a home-manager module to configure waywall
  home.file = {
    ".config/waywall" = {
      enable = true;
      # recursive = true;
      # -- FIXME: once the waywall config is stable put it into store and thereby make it immutable
      source = config.lib.file.mkOutOfStoreSymlink "/persist/nixos/home/jacobsen/programs/minecraft/waywall";
    };
  };
}
