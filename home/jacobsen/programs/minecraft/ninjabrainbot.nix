{
  pkgs,
  ...
}:
{
  home.packages = [
    pkgs.ninjabrainbot
  ];

  # TODO: create a home-manager module to configure ninjabrainbot
  preservation.preserveAt.state-dir.files = [
    ".java/.userPrefs/ninjabrainbot/prefs.xml"
  ];
}
