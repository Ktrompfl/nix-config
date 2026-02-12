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
  version = "unstable-2026-02-12";

  src = fetchFromGitHub {
    owner = "mahkoh";
    repo = "jay";
    rev = "346c6a7345e4295ddf64e2b6f823e7f8741d056b";
    sha256 = "sha256-w9+VlzWAQ2UYxlLuIIbaYWqCLNOJAATgo8/Qntyr/hA=";
  };

  cargoHash = "sha256-NnuNgEwLpgrxA3TtaJRfb9GX/5hspfM0vNPUNjMBTGA=";

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
