{
  config,
  inputs,
  pkgs,
  ...
}:
{
  imports = [ inputs.spicetify-nix.hjemModules.default ];

  programs.spicetify =
    let
      spicePkgs = inputs.spicetify-nix.legacyPackages.${pkgs.stdenv.hostPlatform.system};
    in
    {
      enable = true;
      wayland = true;
      enabledExtensions = with spicePkgs.extensions; [
        fullAppDisplay
        history
        hidePodcasts
        playNext
        volumePercentage
        showQueueDuration
        copyToClipboard
      ];

      theme = spicePkgs.themes.sleek;

      customColorScheme = with config.theme.colors.withoutHashtag; {
        # background
        main = base00;
        sidebar = base00;
        player = base00;
        shadow = base00;
        main-secondary = base01;
        card = base01;
        tab-active = base01;

        # foreground
        text = base05;
        subtext = base04;
        misc = base05;

        # accent
        nav-active = base01;
        nav-active-text = base0D;
        button = base0D;
        button-active = base0D;
        play-button = base0D;
        playback-bar = base0D;
        button-secondary = base03;
        button-disabled = base02;
        selected-row = base04;
        notification = base02;
        notification-error = base08;
      };
    };

  preservation.preserveAt.state-dir.directories = [ ".config/spotify" ];
}
