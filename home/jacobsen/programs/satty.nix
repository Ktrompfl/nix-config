{ config, ... }: {
  programs.satty = {
    enable = true;
    settings = {
      general = {
        fullscreen = true;
        auto-copy = true;
        copy-command = "wl-copy";
        output-filename = "~/Pictures/screenshots/%Y-%m-%d-%H%M%S.png";
        initial-tool = "crop";
        actions-on-enter = [
          "save-to-clipboard"
          "save-to-file"
          "exit"
        ];
        actions-on-escape = [
          "exit"
        ];
      };
      # use font / color palette from stylix
      font = {
        family = config.stylix.fonts.sansSerif.name;
        style = "Regular";
      };
      color-palette = {
        palette = with config.lib.stylix.colors.withHashtag; [
          "${base08}ff" # red     - errors/highlights
          "${base0B}ff" # green   - success/ok
          "${base0A}ff" # yellow  - warnings
          "${base0D}ff" # blue    - info/accent
          "${base0E}ff" # purple  - extra accent
        ];
      };
    };
  };
}
