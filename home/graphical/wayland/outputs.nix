{ lib, pkgs, ... }:
let
  # shikane only considers a profile when it matches every connected output
  # and each output offers the profile's mode. The fallback copy of each profile
  # leaves the modes alone and searches by regex, which ranks below full text,
  # so it is tried only when the exact profile does not match or is refused.
  withFallback = profile: [
    profile
    {
      inherit (profile) exec;
      name = "${profile.name} (any mode)";
      output = map (
        output:
        removeAttrs output [ "mode" ]
        // {
          search =
            let
              fulltext = builtins.match "([dnmvs]*)=(.*)" output.search;
            in
            if fulltext == null then
              output.search
            else
              "${lib.elemAt fulltext 0}/^${lib.escapeRegex (lib.elemAt fulltext 1)}$";
        }
      ) profile.output;
    }
  ];

  output =
    search: attrs:
    {
      inherit search;
      enable = true;
      scale = 1.0;
      transform = "normal";
    }
    // attrs;

  panel = output "n=eDP-1" {
    mode = "1920x1080@60.049Hz";
    position = "0,0";
  };

  vg270u = output "m=VG270U P" {
    mode = "2560x1440@143.995Hz";
    position = "0,410";
  };

  benq = output "m=BenQ GL2480" {
    mode = "1920x1080@74.973Hz";
    position = "2560,0";
    scale = 102.0 / 120.0;
    transform = "90";
  };

  # any other output next to the laptop panel, which the compositor mirrors
  external = output "n/." {
    mode = "1920x1080@60Hz";
    position = "0,1080";
  };

  profiles = [
    {
      name = "desktop";
      output = [
        vg270u
        benq
      ];
    }
    {
      name = "desktop horizontal";
      output = [ vg270u ];
    }
    {
      name = "desktop vertical";
      output = [ benq ];
    }
    {
      name = "laptop";
      output = [ panel ];
    }
    {
      name = "laptop external";
      output = [
        panel
        external
      ];
    }
  ];

  restoreWallpaper = profile: profile // { exec = [ "${lib.getExe' pkgs.awww "awww"} restore" ]; };

  configFile = (pkgs.formats.toml { }).generate "shikane.toml" {
    profile = lib.concatMap (profile: withFallback (restoreWallpaper profile)) profiles;
  };
in
{
  packages = [ pkgs.shikane ];

  xdg.config.files."shikane/config.toml".source = configFile;

  wayland.services.shikane = {
    description = "Output configuration";
    # the path changes with the profiles, which restarts the service
    serviceConfig.ExecStart = "${lib.getExe pkgs.shikane} --config ${configFile}";
    slice = "session";
  };
}
