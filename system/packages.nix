{ pkgs, ... }:
{
  environment.systemPackages = with pkgs; [
    curl
    fd
    git
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
