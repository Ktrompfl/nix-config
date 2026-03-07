{
  inputs,
  pkgs,
  lib,
  libGL,
  libinput,
  pkgconf,
  xkeyboard_config,
  libgbm,
  pango,
  udev,
  shaderc,
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

  SHADERC_LIB_DIR = "${lib.getLib shaderc}/lib";
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
    udev
    libinput
    shaderc
  ];

  runtimeDependencies = [
    libglvnd
    vulkan-loader
  ];

  cargoTestExtraArgs =
    let
      # The following tests fail in the sandboxed build environment and must be disabled.
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
    # install desktop portal
    install -D etc/jay.portal $out/share/xdg-desktop-portal/portals/jay.portal
    install -D etc/jay-portals.conf $out/share/xdg-desktop-portal/jay-portals.conf

    # install desktop entry for display managers
    install -D etc/jay.desktop $out/share/wayland-sessions/jay.desktop

    # install shell completions
    installShellCompletion --cmd jay \
      --bash <($out/bin/jay generate-completion bash) \
      --fish <($out/bin/jay generate-completion fish) \
      --zsh <($out/bin/jay generate-completion zsh)
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
