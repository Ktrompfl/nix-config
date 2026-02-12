{ config, ... }:
{
  sops.secrets."eduroam/identity" = { };
  sops.secrets."eduroam/password" = { };

  sops.templates.eduroam-env.content = ''
    EDUROAM_IDENTITY=${config.sops.placeholder."eduroam/identity"}
    EDUROAM_PASSWORD=${config.sops.placeholder."eduroam/password"}
  '';

  networking.networkmanager.ensureProfiles.environmentFiles = [
    config.sops.templates.eduroam-env.path
  ];
  networking.networkmanager.ensureProfiles.profiles = {
    eduroam = {
      connection = {
        id = "eduroam";
        type = "802-11-wireless";
        permissions = "user:jacobsen";
      };
      "802-11-wireless" = {
        ssid = "eduroam";
        security = "802-11-wireless-security";
      };
      "802-11-wireless-security" = {
        key-mgmt = "wpa-eap";
        proto = "rsn";
        pairwise = "ccmp";
        group = "ccmp,tkip";
      };
      "802-1x" = {
        eap = "peap";
        phase2-auth = "mschapv2";
        ca-cert = "${./isrg-root-x1.crt}";
        altsubject-matches = "DNS:radius.rptu.de";
        anonymous-identity = "wifi@rptu.de";
        identity = "$EDUROAM_IDENTITY";
        password = "$EDUROAM_PASSWORD";
      };
      ipv4.method = "auto";
      ipv6.method = "auto";
    };
  };
}
