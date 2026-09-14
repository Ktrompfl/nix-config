{
  imports = [
    ./desktop
    ./services
    ./wayland

    # sandboxed through the apps module
    ./apps/firefox

    ./apps/audio.nix
    ./apps/chromium.nix
    ./apps/discord.nix
    ./apps/imv.nix
    ./apps/libreoffice.nix
    ./apps/mpv.nix
    ./apps/satty.nix
    ./apps/seafile.nix
    ./apps/signal.nix
    ./apps/spotify.nix
    ./apps/thunderbird.nix
    ./apps/zathura.nix
    ./apps/zotero.nix

    # not sandboxed
    # ./programs/vscode
    ./programs/better-control.nix
    ./programs/zed.nix
  ];
}
