{
  accounts.email.accounts =
    let
      mkEmail = domain: user: "${user}@${domain}";
    in
    {
      Perpendicularity = {
        address = mkEmail "perpendicularity.xyz" "jacobsen";
        aliases = [ (mkEmail "mailbox.org" "nicolausjacobsen") ];
        primary = true;
        imap = {
          host = "imap.mailbox.org";
          port = 993;
        };
        smtp = {
          host = "smtp.mailbox.org";
          port = 587;
          tls.useStartTls = true;
        };
        thunderbird.enable = true;
        realName = "Nicolaus Jacobsen";
        userName = mkEmail "perpendicularity.xyz" "jacobsen";
      };
      CTM = {
        address = mkEmail "mathematik.uni-kl.de" "jacobsen";
        imap = {
          host = "mail.mathematik.uni-kl.de";
          port = 993;
        };
        thunderbird.enable = true;
        realName = "Nicolaus Jacobsen";
        userName = "jacobsen";
      };
      RPTU = {
        address = mkEmail "rptu.de" "jacobsen";
        imap = {
          host = "mail.rptu.de";
          port = 993;
        };
        smtp = {
          host = "smtp.rptu.de";
          port = 587;
          tls.useStartTls = true;
        };
        thunderbird.enable = true;
        realName = "Nicolaus Jacobsen";
        userName = mkEmail "rptu.de" "jacobsen";
      };
    };
}
