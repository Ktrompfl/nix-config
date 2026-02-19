{
  lib,
  makeRustPlatform,
  fetchFromGitHub,
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
  rust-bin,
}:
let
  # requires unstable rust features
  rustPlatform = makeRustPlatform {
    cargo = rust-bin.selectLatestNightlyWith (toolchain: toolchain.default);
    rustc = rust-bin.selectLatestNightlyWith (toolchain: toolchain.default);
  };
in
rustPlatform.buildRustPackage (finalAttrs: {
  pname = "jay";
  version = "unstable-2026-02-19";

  src = fetchFromGitHub {
    owner = "mahkoh";
    repo = "jay";
    rev = "f0c78c3fe6c31bdb7a0912b6748e4ffe22a9bf16";
    sha256 = "sha256-TMleLTCzlzauYDOTfS2trhs7n5bXeUsdXMNi3Ydqvbc=";
  };

  cargoHash = "sha256-h9fky7UHZ7PxNUEmZ+J2/QjaRT4oJ/WWNZW1i4D2WMA=";

  SHADERC_LIB_DIR = "${lib.getLib shaderc}/lib";

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

  checkFlags = [
    # the following tests fail in the sandboxed build environment and must be disabled
    "--skip=cpu_worker::tests::cancel"
    "--skip=cpu_worker::tests::complete"
    "--skip=io_uring::ops::read_write_no_cancel::tests::cancel_in_kernel"
    "--skip=io_uring::ops::read_write_no_cancel::tests::cancel_in_userspace"
  ];

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
})
