{
  lib,
  pkgs,
  inputs,
}:
let
  craneLib = inputs.crane.mkLib pkgs;

  libraries = with pkgs; [
    cairo
    glib
    libdrm
    libgbm
    libinput
    pango
    seatd
    udev
    vulkan-loader
  ];

  # the protocol XML the wire crate generates its bindings from
  protocols = pkgs.symlinkJoin {
    name = "getaway-protocols";
    paths = with pkgs; [
      "${wayland-scanner}/share"
      "${wayland-protocols}/share"
      "${wlr-protocols}/share"
      "${kdePackages.plasma-wayland-protocols}/share"
      (runCommand "wlroots-protocols" { } ''
        mkdir -p $out/wlroots
        cp ${wlroots.src}/protocol/virtual-keyboard-unstable-v1.xml $out/wlroots/
      '')
    ];
  };

  desktop = pkgs.writeText "getaway.desktop" ''
    [Desktop Entry]
    Name=getaway
    Comment=A wayland compositor
    Exec=getaway
    Type=Application
    DesktopNames=getaway
  '';

  portals = pkgs.writeText "getaway-portals.conf" ''
    [preferred]
    default=gtk
    org.freedesktop.impl.portal.ScreenCast=wlr
    org.freedesktop.impl.portal.Screenshot=wlr
  '';

  # only what cargo builds: anything else here (nix files, notes, untracked files)
  # neither breaks nor rebuilds the package
  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./backend
      ./geom
      ./getaway
      ./gfx
      ./kms
      ./nested
      ./server
      ./wire
      ./wm
      ./wm-tree
    ];
  };

  # dependencies build once into their own derivation; workspace changes reuse it
  common = {
    pname = "getaway";
    version = "0.1.0";
    inherit src;
    strictDeps = true;
    cargoExtraArgs = "--locked --package getaway";
    nativeBuildInputs = with pkgs; [
      pkg-config
      shaderc
    ];
    buildInputs = libraries;
    PROTOCOL_DIR = protocols;
    GETAWAY_XKB_ROOT = "${pkgs.xkeyboard-config}/share/X11/xkb";
    doCheck = false;
  };
in
craneLib.buildPackage (
  common
  // {
    cargoArtifacts = craneLib.buildDepsOnly common;
    postInstall = ''
      install -Dm644 ${desktop} $out/share/wayland-sessions/getaway.desktop
      install -Dm644 ${portals} $out/share/xdg-desktop-portal/getaway-portals.conf
    '';
    postFixup = "patchelf --add-rpath ${pkgs.vulkan-loader}/lib $out/bin/getaway";
    passthru = {
      providedSessions = [ "getaway" ];
      inherit libraries protocols;
    };
    meta = {
      description = "A wayland compositor";
      mainProgram = "getaway";
      platforms = lib.platforms.linux;
    };
  }
)
