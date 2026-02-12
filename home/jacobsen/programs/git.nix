{ lib, ... }:
{
  programs.git = {
    enable = true;
    settings = {
      user.name = "Nicolaus Jacobsen";
      user.email = builtins.concatStringsSep "@" [
        "jacobsen"
        "rptu.de"
      ];
      init.defaultBranch = "main";
    };
    ignores = lib.strings.splitString "\n" ''
      ### archive
      # It's better to unpack these files and commit the raw source because
      # git has its own built in compression methods.
      *.7z
      *.jar
      *.rar
      *.zip
      *.gz
      *.gzip
      *.tgz
      *.bzip
      *.bzip2
      *.bz2
      *.xz
      *.lzma
      *.cab
      *.xar
      *.zst
      *.tzst

      # Packing-only formats
      *.iso
      *.tar

      # Package management formats
      *.dmg
      *.xpi
      *.gem
      *.egg
      *.deb
      *.rpm
      *.msi
      *.msm
      *.msp
      *.txz

      ### backup
      *.bak
      *.gho
      *.ori
      *.orig
      *.tmp

      ### linux
      *~

      # temporary files which can be created if a process still has a handle open of a deleted file
      .fuse_hidden*

      # KDE directory preferences
      .directory

      # Linux trash folder which might appear on any partition or disk
      .Trash-*

      # .nfs files are created when an open file is removed but is still being accessed
      .nfs*

      ### libreoffice
      # LibreOffice locks
      .~lock.*#

      ### vim
      # Swap
      [._]*.s[a-v][a-z]
      !*.svg  # comment out if you don't need vector files
      [._]*.sw[a-p]
      [._]s[a-rt-v][a-z]
      [._]ss[a-gi-z]
      [._]sw[a-p]

      # Session
      Session.vim
      Sessionx.vim

      # Temporary
      .netrwhist
      *~
      # Auto-generated tag files
      tags
      # Persistent undo
      [._]*.un~

      ### vs code
      .vscode/*
      !.vscode/settings.json
      !.vscode/tasks.json
      !.vscode/launch.json
      !.vscode/extensions.json
      !.vscode/*.code-snippets

      # Local History for Visual Studio Code
      .history/

      # Built Visual Studio Code Extensions
      *.vsix
    '';
  };

}
