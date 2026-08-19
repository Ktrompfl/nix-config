let
  browser = "firefox.desktop";
  editor = "dev.zed.Zed.desktop";
  images = "viewnior.desktop";
  media = "vlc.desktop";
  reader = "org.pwmt.zathura-pdf-mupdf.desktop";
in
{
  xdg.mime-apps.default-applications = {
    "application/pdf" = reader;

    "image/gif" = images;
    "image/jpeg" = images;
    "image/png" = images;
    "image/svg" = browser;

    "audio/aac" = media;
    "audio/flac" = media;
    "audio/mp3" = media;
    "audio/wav" = media;

    "video/avi" = media;
    "video/mkv" = media;
    "video/mp4" = media;

    "text/plain" = editor;
    "text/markdown" = editor;
    "text/x-csrc" = editor;
    "text/x-chdr" = editor;
    "text/x-c++src" = editor;
    "text/x-python" = editor;
    "text/x-lua" = editor;
    "text/x-rust" = editor;
    "text/x-haskell" = editor;
    "text/x-julia" = editor;
    "text/x-tex" = editor;
    "text/x-nix" = editor;
    "text/x-shellscript" = editor;
    "text/css" = editor;
    "text/csv" = editor;
    "application/json" = editor;
    "application/toml" = editor;
    "application/x-yaml" = editor;
    "application/xml" = editor;
    "application/javascript" = editor;
    "x-scheme-handler/zed" = editor;

    "text/html" = browser;
    "application/xhtml+xml" = browser;
    "application/x-extension-htm" = browser;
    "application/x-extension-html" = browser;
    "application/x-extension-xht" = browser;
    "application/x-extension-xhtml" = browser;
    "x-scheme-handler/about" = browser;
    "x-scheme-handler/chrome" = browser;
    "x-scheme-handler/http" = browser;
    "x-scheme-handler/https" = browser;
    "x-scheme-handler/unknown" = browser;
  };
}
