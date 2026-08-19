# search.json.mozlz4
#
# Firefox stores its engines mozlz4-compressed, and guards the default-engine
# choice with a hash over the profile path, the engine id and a fixed
# disclaimer. Writing the file without that hash makes Firefox discard the
# choice, so the hash is computed here the same way Firefox does.
{
  lib,
  runCommand,
  mozlz4a,
  openssl,
  firefox,
}:
{
  # The profile *directory name*, which is what Firefox salts the hash with.
  profilePath,
  engines,
  default ? null,
  privateDefault ? null,
  order ? [ ],
}:
let
  # Firefox distinguishes engines it ships from ones the profile defines. An
  # entry with no `urls` is taken to be app-provided and carries nothing but
  # its id and metadata; anything else is spelled out in full.
  buildEngine =
    name: engine:
    let
      isAppProvided = !(engine ? urls);

      metaData =
        (engine.metaData or { })
        // lib.optionalAttrs (lib.elem name order) {
          order = 1 + lib.lists.findFirstIndex (e: e == name) 0 order;
        };

      # `icon` is a single path or URL; `iconMapObj` gives sizes explicitly.
      iconMapObj =
        lib.optionalAttrs (engine ? icon) {
          "16" = if lib.hasPrefix "http" engine.icon then engine.icon else "file://${engine.icon}";
        }
        // engine.iconMapObj or { };
    in
    if isAppProvided then
      {
        id = name;
        _isAppProvided = true;
        _metaData = metaData;
      }
    else
      {
        id = name;
        _name = name;
        _isAppProvided = false;
        _metaData = metaData;
        _urls = engine.urls;
        _definedAliases = engine.definedAliases or [ ];
        _loadPath = "[hjem]/programs.firefox.profile.search.engines.${builtins.toJSON name}";
      }
      // lib.optionalAttrs (iconMapObj != { }) { _iconMapObj = iconMapObj; }
      // lib.optionalAttrs (engine ? updateInterval) { _updateInterval = engine.updateInterval; };

  # App-provided engines sort first, then the rest by name, which is the order
  # Firefox itself writes.
  built = lib.mapAttrsToList buildEngine engines;
  sorted =
    lib.filter (e: e._isAppProvided && e._metaData ? order) built
    ++ lib.filter (e: !e._isAppProvided) built
    ++ lib.filter (e: e._isAppProvided && !(e._metaData ? order)) built;
  # Firefox's own wording, checked byte-for-byte against what it writes; the
  # application name is substituted in from the package below.
  disclaimer =
    "By modifying this file, I agree that I am doing so "
    + "only within @appName@ itself, using official, user-driven search "
    + "engine selection processes, and in a way which does not circumvent "
    + "user consent. I acknowledge that any attempt to change this file "
    + "from outside of @appName@ is a malicious act, and will be responded "
    + "to accordingly.";

  settings = {
    version = 12;
    engines = sorted;

    metaData =
      lib.optionalAttrs (default != null) {
        defaultEngineId = default;
        defaultEngineIdHash = "@hash@";
      }
      // lib.optionalAttrs (privateDefault != null) {
        privateDefaultEngineId = privateDefault;
        privateDefaultEngineIdHash = "@privateHash@";
      }
      // {
        useSavedOrder = order != [ ];
      };
  };
in
runCommand "search.json.mozlz4"
  {
    nativeBuildInputs = [
      mozlz4a
      openssl
    ];
    json = builtins.toJSON settings;
    salt = lib.optionalString (default != null) (profilePath + default + disclaimer);
    privateSalt = lib.optionalString (privateDefault != null) (
      profilePath + privateDefault + disclaimer
    );
  }
  ''
    # The disclaimer names the application, which Firefox reads from its own
    # application.ini rather than hard-coding.
    applicationIni="$(find ${lib.escapeShellArg firefox} -maxdepth 3 -path ${lib.escapeShellArg firefox}'/lib/*/application.ini' -print -quit)"
    appName="$(sed -n 's/^Name=\(.*\)$/\1/p' "$applicationIni" | head -n1)"

    salt=''${salt//@appName@/"$appName"}
    privateSalt=''${privateSalt//@appName@/"$appName"}

    if [[ -n $salt ]]; then
      export hash=$(echo -n "$salt" | openssl dgst -sha256 -binary | base64)
      export privateHash=$(echo -n "$privateSalt" | openssl dgst -sha256 -binary | base64)
      mozlz4a <(substituteStream json search.json.in --subst-var hash --subst-var privateHash) "$out"
    else
      mozlz4a <(echo "$json") "$out"
    fi
  ''
