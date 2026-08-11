{ pkgs, ... }:
{
  environment.systemPackages = with pkgs.rocmPackages; [
    rocminfo
    rocm-smi
  ];

  nixpkgs.config.rocmSupport = true;

  # ROCm / HIP
  # Note: this tree serves system ROCm consumers only. Julia's Reactant.jl does
  # NOT use it -- its ROCm artifact vendors its own libhsa-runtime64,
  # libamdhip64, librocblas and libhipblaslt, and dlopens nothing from here.
  systemd.tmpfiles.rules =
    let
      rocmEnv = pkgs.symlinkJoin {
        name = "rocm-combined";
        paths = with pkgs.rocmPackages; [
          clr # libamdhip64, libhiprtc
          rocm-runtime # libhsa-runtime64
          rocm-device-libs
          rocalution
          rocblas
          rocfft
          rocrand
          rocsolver
          rocsparse
          roctracer
          miopen
          # hip* wrappers: the portable API layer most ROCm apps link against
          hipblas
          hipblaslt
          hipfft
          hiprand
          hipsolver
          hipsparse
        ];
      };
    in
    [
      "L+    /opt/rocm       -    -    -     -    ${rocmEnv}"
    ];

  environment.sessionVariables = {
    # The RX 7600 XT is gfx1102 (Navi 33); report it as gfx1100 (Navi 31).
    # Both are RDNA3 and ISA-compatible (the device itself advertises the
    # gfx11-generic ISA), but gfx1102 is missing from the target lists of
    # - XLA's ROCm PJRT plugin (used by Reactant.jl), which otherwise rejects
    #   the device outright and silently falls back to CPU, and
    # - hipBLASLt, which ships Tensile kernels only for gfx1100/gfx1101.
    HSA_OVERRIDE_GFX_VERSION = "11.0.0"; # with 11.0.2 the RX 7600 XT is not recognized
  };

  users.users.jacobsen.extraGroups = [
    "render"
    "video"
  ];
}
