{ config, ... }:
let
  fwmark = 51820;
in
{
  # manage with:
  #   networkctl up wg-rptu
  #   networkctl down wg-rptu
  #   networkctl status wg-rptu
  sops.secrets = {
    "wireguard/rptu/hallandren/private-key" = {
      group = "systemd-network";
      mode = "0440";
      restartUnits = [ "systemd-networkd.service" ];
    };
    "wireguard/rptu/hallandren/public-key" = {
      group = "systemd-network";
      mode = "0440";
      restartUnits = [ "systemd-networkd.service" ];
    };
  };

  boot.kernel.sysctl."net.ipv4.conf.all.src_valid_mark" = 1;
  networking.firewall.checkReversePath = "loose";

  systemd.network = {
    netdevs."30-wg-rptu" = {
      netdevConfig = {
        Name = "wg-rptu";
        Kind = "wireguard";
        MTUBytes = "1280";
      };

      wireguardConfig = {
        PrivateKeyFile = config.sops.secrets."wireguard/rptu/hallandren/private-key".path;
        FirewallMark = fwmark;
        RouteTable = fwmark;
      };

      wireguardPeers = [
        {
          PublicKeyFile = config.sops.secrets."wireguard/rptu/hallandren/public-key".path;
          Endpoint = "vpnwg.uni-kl.de:51820";
          AllowedIPs = [
            "0.0.0.0/0"
            "::/0"
          ];
          PersistentKeepalive = 25;
        }
      ];
    };

    networks."30-wg-rptu" = {
      matchConfig.Name = "wg-rptu";
      address = [
        "172.27.2.193/32"
        "2001:638:208:fd49:9a:82ff:fed0:684f/128"
      ];
      dns = [
        "2001:638:208:9::116"
        "2001:638:208:1::116"
        "131.246.9.116"
        "131.246.1.116"
      ];
      domains = [
        "~rptu.de"
        "~uni-kl.de"
      ];
      networkConfig = {
        DNSDefaultRoute = false;
        DNSOverTLS = false;
        DNSSEC = false;
        IPv6AcceptRA = false;
        LinkLocalAddressing = "no";
      };
      linkConfig = {
        ActivationPolicy = "manual";
        RequiredForOnline = false;
      };
      routingPolicyRules = [
        {
          Family = "both";
          Table = "main";
          SuppressPrefixLength = 0;
          Priority = 32764;
        }
        {
          Family = "both";
          FirewallMark = fwmark;
          InvertRule = true;
          Table = fwmark;
          Priority = 32765;
        }
      ];
    };
  };
}
