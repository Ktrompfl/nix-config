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
  version = "unstable-2026-03-05";

  src = fetchFromGitHub {
    owner = "mahkoh";
    repo = "jay";
    rev = "1570ba6b58bce87ce9ab57b30b267eafadfd3bee";
    sha256 = "sha256-rxBNw2G9CJkG1ILiUeB90ZT5QC8BQjt53OGDj3ktbn4=";
  };

  cargoHash = "sha256-Zhf+GwhM6lnx07eUiumBpIuU0BT8DKOEQiIq+c2ColE=";

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
    "--skip=eventfd_cache::tests::test"
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
