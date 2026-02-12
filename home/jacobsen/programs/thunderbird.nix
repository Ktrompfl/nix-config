{
  programs.thunderbird = {
    enable = true;
    profiles.jacobsen.isDefault = true;
    settings = {
      "browser.display.use_system_colors" = true;
      "browser.display.document_color_use" = 2; # always use system colors, ignore contrast
    };
  };

  # thunderbird persistent storage, maybe check if really everything in there is needed
  preservation.preserveAt.data-dir.directories = [ ".thunderbird" ];
}
