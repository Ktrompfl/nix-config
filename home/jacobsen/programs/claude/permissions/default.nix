{ config, ... }:
{
  programs.claude-code.settings.permissions = {
    allow = import ./allow.nix { inherit config; };
    ask = import ./ask.nix null;
    deny = import ./deny.nix null;
  };
}
