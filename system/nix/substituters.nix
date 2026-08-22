{
  nix.settings = {
    substituters = [
      # cache.nixos.org is available as default, priority 40
      "https://nix-community.cachix.org" # priority 41
      "https://cache.numtide.com" # priority 50 (default)
    ];
    trusted-public-keys = [
      "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs="
      "niks3.numtide.com-1:DTx8wZduET09hRmMtKdQDxNNthLQETkc/yaX7M4qK0g="
    ];
  };
}
