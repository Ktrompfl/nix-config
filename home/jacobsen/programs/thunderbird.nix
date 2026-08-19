{
  programs.thunderbird = {
    enable = true;
    profile = "jacobsen";

    settings = {
      "browser.display.use_system_colors" = true;
      "browser.display.document_color_use" = 2; # always use system colors, ignore contrast
      "mailnews.mark_message_read.auto" = false;
    };

    accounts =
      let
        mkEmail = domain: user: "${user}@${domain}";
        realName = "Nicolaus Jacobsen";
      in
      {
        Perpendicularity = {
          inherit realName;
          primary = true;
          address = mkEmail "perpendicularity.xyz" "jacobsen";
          aliases = [ (mkEmail "mailbox.org" "nicolausjacobsen") ];
          userName = mkEmail "perpendicularity.xyz" "jacobsen";
          imap.host = "imap.mailbox.org";
          smtp.host = "smtp.mailbox.org";
        };

        CTM = {
          inherit realName;
          address = mkEmail "mathematik.uni-kl.de" "jacobsen";
          userName = "jacobsen";
          imap.host = "mail.mathematik.uni-kl.de";
        };

        RPTU = {
          inherit realName;
          address = mkEmail "rptu.de" "jacobsen";
          userName = mkEmail "rptu.de" "jacobsen";
          imap.host = "mail.rptu.de";
          smtp.host = "smtp.rptu.de";
        };
      };
  };

  # thunderbird persistent storage, maybe check if really everything in there is needed
  preservation.preserveAt.data-dir.directories = [ ".thunderbird" ];
}
