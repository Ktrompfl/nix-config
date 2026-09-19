{
  generators,
  lib,
  pkgs,
  ...
}:
let
  profile = "jacobsen";

  realName = "Nicolaus Jacobsen";
  mkEmail = domain: user: "${user}@${domain}";

  declared = {
    Perpendicularity = {
      primary = true;
      address = mkEmail "perpendicularity.xyz" "jacobsen";
      aliases = [ (mkEmail "mailbox.org" "nicolausjacobsen") ];
      userName = mkEmail "perpendicularity.xyz" "jacobsen";
      imap.host = "imap.mailbox.org";
      smtp.host = "smtp.mailbox.org";
    };

    CTM = {
      address = mkEmail "mathematik.uni-kl.de" "jacobsen";
      userName = "jacobsen";
      imap.host = "mail.mathematik.uni-kl.de";
    };

    RPTU = {
      address = mkEmail "rptu.de" "jacobsen";
      userName = mkEmail "rptu.de" "jacobsen";
      imap.host = "mail.rptu.de";
      smtp.host = "smtp.rptu.de";
    };
  };

  settings = {
    "widget.use-xdg-desktop-portal.file-picker" = 1; # required inside jail
    "browser.display.use_system_colors" = true;
    "browser.display.document_color_use" = 2; # always use system colors, ignore contrast
    "mailnews.mark_message_read.auto" = false;
  };

  idOf = name: builtins.hashString "sha256" name;

  accounts = lib.mapAttrsToList (
    name: account:
    {
      inherit realName;
      primary = false;
      aliases = [ ];
      signature = "";
      smtp = null;
    }
    // account
    // {
      inherit name;
      id = idOf name;
      imap = {
        port = 993;
      }
      // account.imap;
      smtp = if account ? smtp then { port = 587; } // account.smtp else null;
    }
  ) declared;

  aliasesOf =
    account:
    map (address: {
      inherit address;
      id = idOf address;
      inherit (account) realName;
      smtpOf = account;
    }) account.aliases;

  allAliases = lib.concatMap aliasesOf accounts;
  withSmtp = lib.filter (account: account.smtp != null) accounts;

  identitiesOf =
    account:
    lib.concatStringsSep "," (
      [ "id_${account.id}" ] ++ map (alias: "id_${alias.id}") (aliasesOf account)
    );

  serverPrefs = account: {
    "mail.account.account_${account.id}.server" = "server_${account.id}";
    "mail.account.account_${account.id}.identities" = identitiesOf account;

    "mail.server.server_${account.id}.directory-rel" = "[ProfD]ImapMail/${account.id}";
    "mail.server.server_${account.id}.hostname" = account.imap.host;
    "mail.server.server_${account.id}.port" = account.imap.port;
    "mail.server.server_${account.id}.name" = account.name;
    "mail.server.server_${account.id}.type" = "imap";
    "mail.server.server_${account.id}.userName" = account.userName;
    "mail.server.server_${account.id}.login_at_startup" = true;
    # 3 is SSL/TLS, which is what port 993 implies.
    "mail.server.server_${account.id}.socketType" = 3;
  };

  identityPrefs =
    account:
    {
      "mail.identity.id_${account.id}.fullName" = account.realName;
      "mail.identity.id_${account.id}.useremail" = account.address;
      "mail.identity.id_${account.id}.valid" = true;
      "mail.identity.id_${account.id}.htmlSigText" = account.signature;
    }
    // lib.optionalAttrs (account.smtp != null) {
      "mail.identity.id_${account.id}.smtpServer" = "smtp_${account.id}";
    };

  aliasPrefs =
    alias:
    {
      "mail.identity.id_${alias.id}.fullName" = alias.realName;
      "mail.identity.id_${alias.id}.useremail" = alias.address;
      "mail.identity.id_${alias.id}.valid" = true;
      "mail.identity.id_${alias.id}.htmlSigText" = "";
    }
    // lib.optionalAttrs (alias.smtpOf.smtp != null) {
      "mail.identity.id_${alias.id}.smtpServer" = "smtp_${alias.smtpOf.id}";
    };

  smtpPrefs = account: {
    "mail.smtpserver.smtp_${account.id}.hostname" = account.smtp.host;
    "mail.smtpserver.smtp_${account.id}.port" = account.smtp.port;
    "mail.smtpserver.smtp_${account.id}.username" = account.userName;
    # 3 is password-cleartext over an encrypted link; 2 is STARTTLS.
    "mail.smtpserver.smtp_${account.id}.authMethod" = 3;
    "mail.smtpserver.smtp_${account.id}.try_ssl" = 2;
  };

  primary = lib.findFirst (account: account.primary) (lib.head accounts) accounts;

  accountPrefs = lib.mergeAttrsList (
    map serverPrefs accounts
    ++ map identityPrefs accounts
    ++ map smtpPrefs withSmtp
    ++ map aliasPrefs allAliases
    ++ [
      {
        "mail.accountmanager.accounts" = lib.concatStringsSep "," (
          map (account: "account_${account.id}") accounts ++ [ "account1" ]
        );
        "mail.accountmanager.defaultaccount" = "account_${primary.id}";
        "mail.smtpservers" = lib.concatMapStringsSep "," (account: "smtp_${account.id}") withSmtp;
        "mail.smtp.defaultserver" = "smtp_${primary.id}";
        "mail.openpgp.allow_external_gnupg" = false;
      }
    ]
  );
in
{
  apps.thunderbird = {
    package = pkgs.thunderbird;
    appId = "org.mozilla.thunderbird";

    backup = true;

    # attachments are saved to disk directly; adding them goes through the portal
    access.download = "rw";

    jail.permissions =
      c: with c; [
        gui
        gpu
        network
        notifications
      ];

    files = {
      ".thunderbird/profiles.ini" = {
        generator = generators.toMozillaProfiles;
        value.name = profile;
      };

      ".thunderbird/${profile}/user.js" = {
        generator = generators.toMozillaPrefs;
        value = settings // accountPrefs;
      };
    };
  };
}
