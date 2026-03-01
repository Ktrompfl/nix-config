{ pkgs, ... }:
{
  # ROCm / HIP
  systemd.tmpfiles.rules =
    let
      rocmEnv = pkgs.symlinkJoin {
        name = "rocm-combined";
        paths = with pkgs.rocmPackages; [
          clr
          hipblas
          hipfft
          hiprand
          hipsparse
          hipsolver
          hsakmt
          miopen
          rocblas
          rocfft
          rocminfo
          rocrand
          rocsparse
          rocsolver
          roctracer
        ];
      };
    in
    [
      "L+    /opt/rocm   -    -    -     -    ${rocmEnv}"
    ];

  users.users.jacobsen.extraGroups = [
    "render"
    "video"
  ];
}
