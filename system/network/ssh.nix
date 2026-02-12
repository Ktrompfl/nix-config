{ lib, ... }:
{
  services.openssh = {
    enable = lib.mkDefault true;
    openFirewall = true;
    settings = {
      # securing the ssh server
      AllowUsers = [ "jacobsen" ];
      KbdInteractiveAuthentication = false;
      PasswordAuthentication = false;
      PermitRootLogin = "no";
      TCPKeepAlive = false;
      PermitEmptyPasswords = false;
      PermitTunnel = false;
      # UseDns = false;
      MaxAuthTries = 3;
      MaxSessions = 2;
      ClientAliveInterval = 300;
      ClientAliveCountMax = 0;
      AllowTcpForwarding = false;
      AllowAgentForwarding = false;
      X11Forwarding = false;
      LogLevel = "VERBOSE";
    };
  };
}
