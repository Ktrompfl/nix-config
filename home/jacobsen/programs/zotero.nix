{ pkgs, ... }: {
  home.packages = [ pkgs.zotero ];

  preservation.preserveAt.state-dir.directories = [ "Zotero" ];
}
