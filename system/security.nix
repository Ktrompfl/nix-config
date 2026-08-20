{
  inputs,
  lib,
  pkgs,
  ...
}:
{
  imports = [
    inputs.nix-mineral.nixosModules.nix-mineral
  ];

  # `nh os switch` re-authenticates on every rebuild otherwise. Passwordless
  # activation is not an option: it can activate any configuration, so it is
  # root-equivalent whatever the rule is scoped to.
  security.sudo.extraConfig = ''
    Defaults timestamp_timeout=60
  '';

  # The default limit for open files is defined in /etc/systemd/system.conf
  # as DefaultLimitNOFILE=1024:524288, which is too low for heavy system rebuilds.
  # Increasing the soft limit with PAM limits appears to be the cleanest way.
  security.pam.loginLimits = [
    {
      domain = "*";
      type = "soft";
      item = "nofile";
      value = "65536";
    }
    {
      domain = "*";
      type = "hard";
      item = "nofile";
      value = "1048576";
    }
  ];

  boot.kernelParams = [ "init_on_alloc=1" ];

  nix-mineral = {
    enable = lib.mkDefault true;
    preset = "compatibility";

    # add settings from performance preset
    settings = {
      kernel = {
        # if false, may prevent low resource systems from booting.
        busmaster-bit = true;

        # Enable symmetric multithreading and just use default CPU mitigations,
        # to potentially improve performance.
        # DO NOT disable all cpu mitigations,
        cpu-mitigations = "smt-on";

        # PTI (Page Table Isolation) may tax performance.
        pti = false;

        # Do not enable red zoning and sanity checking with slab debug, since
        # it adds significant memory allocation overhead.
        slab-debug = false;

        # io-uring required for jay compositor
        io-uring = true;

        zero-alloc = false;
      };

      entropy = {
        hwrng = true;
      };

      system = {
        # allow 32-bit libraries and applications to run.
        multilib = true;
      };
    };

    extras = {
      kernel = {
        # Avoid putting trust in the highly privilege ME system,
        # Intel users should read more about the issue at the below links:
        # https://www.kernel.org/doc/html/latest/driver-api/mei/mei.html
        # https://en.wikipedia.org/wiki/Intel_Management_Engine#Security_vulnerabilities
        # https://www.kicksecure.com/wiki/Out-of-band_Management_Technology#Intel_ME_Disabling_Disadvantages
        # https://github.com/Kicksecure/security-misc/pull/236#issuecomment-2229092813
        # https://github.com/Kicksecure/security-misc/issues/239
        intelme-kmodules = false;
      };
      misc = {
        # Use an opinionated SSH hardening config. Complies with ssh-audit.
        # Read what everything does first, or else you might get locked out.
        # This, for example, prevents root login AND password based login.
        ssh-hardening = true;
      };

      system = {
        # Lock the root account. Requires another method of privilege escalation, i.e
        # sudo or doas, and declarative accounts to work properly.
        lock-root = true;
      };
    };
  };

  environment.systemPackages = [
    pkgs.lynis # Security auditing tool for Linux, macOS, and UNIX-based systems
    pkgs.vulnix # NixOS vulnerability scanner
  ];
}
