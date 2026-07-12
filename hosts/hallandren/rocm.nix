{ pkgs, ... }:
{
  environment.systemPackages = with pkgs.rocmPackages; [
    rocminfo
    rocm-smi
  ];

  # ROCm / HIP
  systemd.tmpfiles.rules =
    let
      rocmEnv = pkgs.symlinkJoin {
        name = "rocm-combined";
        paths = with pkgs.rocmPackages; [
          # clr # libamdhip64, libhiprtc
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
      # "L+    /opt/rocm/hip   -    -    -     -    ${pkgs.rocmPackages.clr}"
    ];

  users.users.jacobsen.extraGroups = [
    "render"
    "video"
  ];
}
