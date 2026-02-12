{
  networking.useNetworkd = true;

  systemd.network = {
    enable = true;

    # rename interfaces via udev
    links = {
      "10-wired" = {
        matchConfig.PermanentMACAddress = "24:4b:fe:56:dd:a4";
        linkConfig.Name = "wan";
      };
      "20-wireless" = {
        matchConfig.PermanentMACAddress = "9c:fc:e8:84:bf:75";
        linkConfig.Name = "wlan";
      };
    };

    # setup interfaces via networkd
    networks = {
      "10-wired" = {
        matchConfig.Name = "wan";
        linkConfig.RequiredForOnline = "routable";
        networkConfig = {
          DHCP = true;
          IPv6AcceptRA = true;
        };
      };
    };
  };
}
