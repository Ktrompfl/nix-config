{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.programs.thunderbird;

  idOf = name: builtins.hashString "sha256" name;

  accounts = lib.mapAttrsToList (
    name: account:
    account
    // {
      id = idOf name;
      inherit name;
    }
  ) cfg.accounts;

  aliasesOf =
    account:
    map (address: {
      inherit address;
      id = idOf address;
      inherit (account) realName;
      smtpOf = account;
    }) account.aliases;

  allAliases = lib.concatMap aliasesOf accounts;
  withSmtp = lib.filter (a: a.smtp != null) accounts;

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

  identitiesOf =
    account:
    lib.concatStringsSep "," ([ "id_${account.id}" ] ++ map (a: "id_${a.id}") (aliasesOf account));

  smtpPrefs = account: {
    "mail.smtpserver.smtp_${account.id}.hostname" = account.smtp.host;
    "mail.smtpserver.smtp_${account.id}.port" = account.smtp.port;
    "mail.smtpserver.smtp_${account.id}.username" = account.userName;
    # 3 is password-cleartext over an encrypted link; 2 is STARTTLS.
    "mail.smtpserver.smtp_${account.id}.authMethod" = 3;
    "mail.smtpserver.smtp_${account.id}.try_ssl" = 2;
  };

  primary = lib.findFirst (a: a.primary) (lib.head accounts) accounts;

  accountPrefs = lib.mergeAttrsList (
    map serverPrefs accounts
    ++ map identityPrefs accounts
    ++ map smtpPrefs withSmtp
    ++ map aliasPrefs allAliases
    ++ [
      {
        "mail.accountmanager.accounts" = lib.concatStringsSep "," (
          map (a: "account_${a.id}") accounts ++ [ "account1" ]
        );
        "mail.accountmanager.defaultaccount" = "account_${primary.id}";
        "mail.smtpservers" = lib.concatMapStringsSep "," (a: "smtp_${a.id}") withSmtp;
        "mail.smtp.defaultserver" = "smtp_${primary.id}";
        "mail.openpgp.allow_external_gnupg" = false;
      }
    ]
  );
in
{
  options.programs.thunderbird = {
    enable = lib.mkEnableOption "Thunderbird";

    package = lib.mkOption {
      type = lib.types.nullOr lib.types.package;
      default = pkgs.thunderbird;
    };

    profile = lib.mkOption {
      type = lib.types.str;
      default = "default";
      description = "Name of the profile directory under ~/.thunderbird.";
    };

    settings = lib.mkOption {
      type =
        with lib.types;
        attrsOf (oneOf [
          bool
          int
          str
        ]);
      default = { };
      description = "Preferences written to the profile's user.js.";
    };

    accounts = lib.mkOption {
      default = { };
      description = ''
        Mail accounts. Each contributes a server, an identity and optionally an
        SMTP server, all keyed by a hash of the attribute name.
      '';
      type = lib.types.attrsOf (
        lib.types.submodule {
          options = {
            address = lib.mkOption { type = lib.types.str; };
            realName = lib.mkOption { type = lib.types.str; };
            userName = lib.mkOption { type = lib.types.str; };

            primary = lib.mkOption {
              type = lib.types.bool;
              default = false;
              description = "Whether this is the default account.";
            };

            signature = lib.mkOption {
              type = lib.types.lines;
              default = "";
            };

            aliases = lib.mkOption {
              type = lib.types.listOf lib.types.str;
              default = [ ];
              description = "Further addresses that send through this account.";
            };

            imap = lib.mkOption {
              type = lib.types.submodule {
                options = {
                  host = lib.mkOption { type = lib.types.str; };
                  port = lib.mkOption {
                    type = lib.types.port;
                    default = 993;
                  };
                };
              };
            };

            smtp = lib.mkOption {
              default = null;
              type = lib.types.nullOr (
                lib.types.submodule {
                  options = {
                    host = lib.mkOption { type = lib.types.str; };
                    port = lib.mkOption {
                      type = lib.types.port;
                      default = 587;
                    };
                  };
                }
              );
            };
          };
        }
      );
    };
  };

  config = lib.mkIf cfg.enable {
    packages = lib.optional (cfg.package != null) cfg.package;

    files = {
      ".thunderbird/profiles.ini" = {
        generator = lib.generators.toMozillaProfiles;
        value.name = cfg.profile;
      };

      ".thunderbird/${cfg.profile}/user.js" = {
        generator = lib.generators.toMozillaPrefs;
        value = cfg.settings // accountPrefs;
      };
    };
  };
}
