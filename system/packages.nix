{ pkgs, ... }:
{
  environment.systemPackages = with pkgs; [
    curl
    fd
    git
    gparted
    gptfdisk
    inxi
    jq
    killall
    lshw
    wget
    unzip
    zip
  ];
}
