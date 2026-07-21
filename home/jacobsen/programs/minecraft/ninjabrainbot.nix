{
  pkgs,
  ...
}:
{
  home.packages = [
    pkgs.ninjabrainbot
  ];

  # TODO: declare the preferences here
  preservation.preserveAt.state-dir.files = [
    ".java/.userPrefs/ninjabrainbot/prefs.xml"
  ];
}
