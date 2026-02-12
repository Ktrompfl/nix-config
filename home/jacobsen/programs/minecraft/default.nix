{
  config,
  pkgs,
  ...
}:
{
  home.packages = with pkgs; [
    (prismlauncher.override { additionalPrograms = [ waywall ]; }) # modded minecraft launcher
    ninjabrainbot # stronghold calculator
  ];

  home.file = {
    # Install patched GLFW version for Waywall.
    # To use this with a Prismlauncher instance,
    # enable "Native Libraries" and "User system installation of GLFW"
    # in Settings/Workarounds and set the GLFW librariy path respectively.
    ".local/lib64/libglfw.so".source = "${pkgs.glfw-waywall}/lib/libglfw.so";

    ".config/waywall" = {
      enable = true;
      # recursive = true;
      # -- FIXME: once the waywall config is stable put it into store and thereby make it immutable
      source = config.lib.file.mkOutOfStoreSymlink "/persist/nixos/home/programs/minecraft/waywall";
    };
  };

  preservation.preserveAt.state-dir.directories = [
    ".java/.userPrefs/ninjabrainbot" # preferences
  ];
}
