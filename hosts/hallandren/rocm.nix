{ pkgs, ... }:
{
  environment.systemPackages = with pkgs.rocmPackages; [
    rocminfo
    rocm-smi
  ];

  nixpkgs.config.rocmSupport = true;

  # ROCm / HIP
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
        ];
      };
    in
    [
      "L+    /opt/rocm       -    -    -     -    ${rocmEnv}"
    ];

  environment.sessionVariables = {
    HSA_OVERRIDE_GFX_VERSION = "11.0.0"; # 11.0.2 is not supported
  };

  users.users.jacobsen.extraGroups = [
    "render"
    "video"
  ];
}
