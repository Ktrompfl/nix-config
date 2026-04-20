{
  inputs,
  pkgs,
  lib,
  stdenv,
  libGL,
  libinput,
  pkgconf,
  xkeyboard_config,
  libgbm,
  pango,
  udev,
  fontconfig,
  libglvnd,
  vulkan-loader,
  autoPatchelfHook,
  installShellFiles,
  allowRealtimeConfigSO ? false,
  ...
}:
let
  src = inputs.jay;

  # jay requires unstable rust features, use rust nightly
  craneLib = (inputs.crane.mkLib pkgs).overrideToolchain (
    p: p.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default)
  );
in
craneLib.buildPackage {
  inherit src;

  pname = "jay";
  version = "unstable";

  JAY_ALLOW_REALTIME_CONFIG_SO = if allowRealtimeConfigSO then 1 else 0;

  nativeBuildInputs = [
    autoPatchelfHook
    installShellFiles
    pkgconf
  ];

  buildInputs = [
    libGL
    xkeyboard_config
    libgbm
    pango
    fontconfig
    udev
    libinput
  ];

  runtimeDependencies = [
    libglvnd
    vulkan-loader
  ];

  cargoTestExtraArgs =
    let
      # the following tests require access to io_uring, which is disabled in the sandboxed build environment
      skipTests = [
        "cpu_worker::tests::cancel"
        "cpu_worker::tests::complete"
        "eventfd_cache::tests::test"
        "io_uring::ops::read_write_no_cancel::tests::cancel_in_kernel"
        "io_uring::ops::read_write_no_cancel::tests::cancel_in_userspace"
      ];
    in
    # All the arguments following the two dashes (--) are passed to the test binaries
    # and thus to libtest (rustc’s built in unit-test and micro-benchmarking framework).
    "-- ${lib.strings.concatMapStringsSep " " (t: "--skip=${t}") skipTests}";

  postInstall = ''
    install -D etc/jay.portal $out/share/xdg-desktop-portal/portals/jay.portal
    install -D etc/jay-portals.conf $out/share/xdg-desktop-portal/jay-portals.conf
    install -D etc/jay.desktop $out/share/wayland-sessions/jay.desktop
  ''
  + lib.optionalString (stdenv.buildPlatform.canExecute stdenv.hostPlatform) ''
    installShellCompletion --cmd jay \
      --bash <("$out/bin/jay" generate-completion bash) \
      --zsh <("$out/bin/jay" generate-completion zsh) \
      --fish <("$out/bin/jay" generate-completion fish)
  '';

  passthru.providedSessions = [ "jay" ];

  meta = {
    description = "Wayland compositor written in Rust";
    homepage = "https://github.com/mahkoh/jay";
    license = lib.licenses.gpl3;
    platforms = lib.platforms.linux;
    mainProgram = "jay";
  };
}
