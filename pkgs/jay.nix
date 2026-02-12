{
  lib,
  rustPlatform,
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
}:
rustPlatform.buildRustPackage (finalAttrs: {
  pname = "jay";
  version = "unstable-2026-02-09";

  src = fetchFromGitHub {
    owner = "mahkoh";
    repo = "jay";
    rev = "89c9b8d2e0dd05e616e74e40b6eb57e139b2c598";
    sha256 = "sha256-8gENRjQa9yIWITx88TIpZVR23G8H5doDtKOXmrK3tb8=";
  };

  cargoHash = "sha256-r5apvtL+TK/yfsIWP75tWp2bPM5fYj4btocl4H2FtJ4=";

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
    maintainers = with lib.maintainers; [ dit7ya ];
    mainProgram = "jay";
  };
})
