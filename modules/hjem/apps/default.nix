{
  config,
  inputs,
  lib,
  pkgs,
  ...
}:
let
  inherit (lib)
    any
    attrNames
    attrValues
    concatLines
    concatStringsSep
    elem
    filterAttrs
    hasPrefix
    escapeShellArg
    isDerivation
    isPath
    mapAttrsToList
    mkOption
    optional
    types
    unique
    ;

  inherit (builtins) dirOf;

  jail = import ./jail.nix { inherit inputs pkgs; };

  cfg = config.apps;
  homeDirectory = config.directory;

  # A jailed application never appears in the home directory. It gets a private
  # home of its own on the partition `backup` selects, bound over $HOME inside
  # the jail, and nothing is preserved: there is no path in the root filesystem
  # that has to be mapped back onto persistent storage.
  #
  # These live below the preservation storage roots rather than beside them so
  # that they inherit a directory the user already owns, which is what lets the
  # wrapper create them without any privileged tmpfiles rule.
  storageOf =
    name: app: "${if app.backup then "/persist" else "/cache"}${homeDirectory}/apps/${name}";

  # A grant key that names an xdg user directory resolves through its
  # XDG_*_DIR variable, so this cannot disagree with xdg-user-dirs about where
  # that directory is -- `screenshots` really is ~/Pictures/screenshots rather
  # than ~/screenshots. Anything else is a path below the home directory.
  #
  # Either way the result is absolute and below $HOME, so the grant lands on
  # top of the private home and the application finds it where it expects.
  # A key that is neither is not caught here; bwrap refuses to start and names
  # the path it could not find.
  grantedPath =
    key: config.environment.sessionVariables."XDG_${lib.toUpper key}_DIR" or "${homeDirectory}/${key}";

  # desktop/gtk.nix and desktop/qt.nix write the toolkit configuration into the
  # shared home, which a private home by definition does not have. Bind it back
  # read-only -- but from the store rather than from the home directory, since
  # walking the latter at runtime would drag the preservation symlinks and with
  # them the /cache mount point into the jail.
  themingDirectories = [
    "gtk-3.0"
    "gtk-4.0"
    "Kvantum"
    "qt5ct"
    "qt6ct"
  ];

  # The cursor's "default" theme has to sit in the icon search path as a
  # directory, so unlike the toolkit rc files it cannot be named by a variable
  # and has to be bound in.
  themingDataFiles = [ "icons/default/index.theme" ];

  themingFiles =
    mapAttrsToList
      (path: file: {
        path = ".config/${path}";
        inherit (file) source;
      })
      (
        filterAttrs (
          path: _: any (directory: hasPrefix "${directory}/" path) themingDirectories
        ) config.xdg.config.files
      )
    ++ mapAttrsToList (path: file: {
      path = ".local/share/${path}";
      inherit (file) source;
    }) (filterAttrs (path: _: elem path themingDataFiles) config.xdg.data.files);

  # hjem's trio of ways to say what a file contains, kept identical so that
  # moving a declaration from `xdg.config.files` to `apps.<name>.files` is a
  # change of destination only. A generator may return either a string or a
  # derivation; both are accepted.
  # Both paths are relative to the home directory, so the link is written
  # relative too: an absolute one would dangle on the host, where the private
  # home is not mounted over $HOME.
  relativeTo =
    from: to: lib.concatStrings (map (_: "../") (lib.init (lib.splitString "/" from))) + to;

  contentsOf =
    path: file:
    let
      name = lib.strings.sanitizeDerivationName (baseNameOf path);
      generated = file.generator file.value;
    in
    if file.source != null then
      file.source
    else if file.text != null then
      pkgs.writeText name file.text
    else if isPath generated || isDerivation generated then
      generated
    else
      pkgs.writeText name generated;

  permissionsOf =
    name: app:
    let
      storage = storageOf name app;
      managed = lib.filterAttrs (_: file: file.mutable) app.files;
      bound = lib.filterAttrs (_: file: !file.mutable) app.files;

      # Seeded files are removed again once their declaration goes away, which
      # a plain `install` would never notice.
      manifest = pkgs.writeText "${name}-managed" (concatLines (attrNames managed));

      readOnlyFiles =
        mapAttrsToList (path: file: {
          inherit path;
          source = contentsOf path file;
        }) bound
        ++ themingFiles;

      # bwrap will not create a bind destination whose parent is missing, and a
      # private home starts out with nothing in it. Seeded files get their
      # parents from `install -D`; these have to be made by hand.
      parents = unique (
        map (file: "${storage}/${dirOf file.path}") readOnlyFiles
        ++ mapAttrsToList (path: _: "${storage}/${dirOf path}") app.links
        ++ map (path: "${storage}/${path}") app.directories
      );
    in
    c:
    with c;
    [
      (add-runtime ''
        APP_HOME=${escapeShellArg storage}
        mkdir -p "$APP_HOME"
        if [ -f "$APP_HOME/.managed" ]; then
          comm -23 <(sort "$APP_HOME/.managed") <(sort ${manifest}) | while IFS= read -r stale; do
            [ -n "$stale" ] && rm -f "$APP_HOME/$stale"
          done
        fi
        install -D -m644 ${manifest} "$APP_HOME/.managed"
        mkdir -p ${concatStringsSep " " (map escapeShellArg parents)}

        # A read-only bind cannot be made over an existing symlink whose
        # target is not itself in the jail, and a private home migrated from
        # the shared one is full of the links hjem used to manage. bwrap fails
        # the whole sandbox when that happens, so clear the way first.
        rm -f ${concatStringsSep " " (map (file: escapeShellArg "${storage}/${file.path}") readOnlyFiles)}
      '')

      (add-runtime (
        concatLines (
          mapAttrsToList (
            path: target:
            "ln -sfn ${escapeShellArg (relativeTo path target)} ${escapeShellArg "${storage}/${path}"}"
          ) app.links
        )
      ))

      (rw-bind storage homeDirectory)
    ]

    # Managed config is copied in rather than linked or bound. The app can then
    # rewrite it with any method it likes -- truncate, or write-and-rename,
    # which defeats both a symlink into the read-only store and a bind mounted
    # file -- and the declarative value wins again at the next launch.
    #
    # `-p` keeps the store's timestamp rather than stamping the copy with the
    # time of the launch. Anything caching a verdict about a file keys it on
    # the mtime, so without this the work is redone on every start: Firefox
    # re-verified all seven extension signatures each launch, which is about
    # three and a half seconds before it will so much as hand a url to the
    # instance already running.
    ++ mapAttrsToList (
      path: file:
      add-runtime "install -D -p -m${file.mode} ${escapeShellArg "${contentsOf path file}"} ${escapeShellArg "${storage}/${path}"}"
    ) managed

    # The jail clears the environment, so anything the session sets for the
    # toolkits has to be named here. Without them every Qt application falls
    # back to xcb -- with no X server to fall back to -- and every GTK one
    # comes up unthemed.
    ++ [ locale ]

    ++ map try-fwd-env [
      "GTK2_RC_FILES"
      "GTK_A11Y"
      "GTK_PATH"
      "GTK_THEME"
      "NIXOS_OZONE_WL"
      "QT_PLUGIN_PATH"
      "QT_QPA_PLATFORM"
      "QT_QPA_PLATFORMTHEME"
      "QT_STYLE_OVERRIDE"

      # libreoffice and others pick a toolkit backend from this
      "XDG_CURRENT_DESKTOP"
    ]

    # The variables above are only useful if what they point at is in here
    # too, under the name they use.
    ++ map (variable: paths-from-var variable ":") [
      "GTK_PATH"
      "QT_PLUGIN_PATH"
      "XDG_DATA_DIRS"

      # these two name a generated file in the store rather than a directory,
      # and nothing else would pull it into the jail's closure
      "GTK2_RC_FILES"
      "XENVIRONMENT"
    ]

    # Config the app only ever reads needs no copy and leaves no artifact.
    ++ map (file: ro-bind file.source "${homeDirectory}/${file.path}") readOnlyFiles

    ++ mapAttrsToList (
      key: mode: (if mode == "rw" then readwrite else readonly) (grantedPath key)
    ) app.access

    ++ optional (app.appId != null) (portals app.appId)

    ++ app.jail.permissions c;

  # jail.nix wraps a single executable and forwards nothing else, so the
  # desktop entry and icons have to be carried over separately. Without this
  # step `xdg-open` and the launcher would keep resolving the unjailed binary.
  resources =
    name: app: binaries:
    pkgs.runCommand "${name}-resources" { } ''
      ${lib.optionalString (app.appId != null) ''
        # A GApplication registers on the session bus under the id its desktop
        # entry is named after, and the jail only lets it own the id declared
        # here. Getting that wrong is not a build error and not a crash: the
        # program starts, fails to register, and exits with ServiceUnknown.
        # So when the package ships exactly one entry whose name is a legal
        # bus name, hold the declaration to it.
        shipped=$(
          ls "${app.package}/share/applications" 2>/dev/null |
            sed 's/\.desktop$//' |
            grep -E '^[A-Za-z_-][A-Za-z0-9_-]*(\.[A-Za-z_-][A-Za-z0-9_-]*)+$' || true
        )
        if [ "$(printf '%s' "$shipped" | grep -c .)" = 1 ] && [ "$shipped" != "${app.appId}" ]; then
          echo "apps.${name}: appId is '${app.appId}' but the package ships '$shipped.desktop'" >&2
          exit 1
        fi
      ''}
      mkdir -p "$out/share"
      for directory in "${app.package}/share"/*; do
        [ -e "$directory" ] || continue
        case "$(basename "$directory")" in
          applications)
            mkdir -p "$out/share/applications"
            for entry in "$directory"/*.desktop; do
              [ -e "$entry" ] || continue
              sed ${
                concatStringsSep " " (
                  map (binary: "-e 's|${app.package}/bin/${binary}|${binaries}/bin/${binary}|g'") app.binaries
                )
              } \
                  "$entry" > "$out/share/applications/$(basename "$entry")"
            done
            ;;
          *)
            ln -s "$directory" "$out/share/$(basename "$directory")"
            ;;
        esac
      done
    '';

  wrap =
    name: app:
    let
      permissions = permissionsOf name app;

      # Joined first so that the desktop entries below have a single, known
      # path to point at, whichever of the binaries they name.
      binaries = pkgs.symlinkJoin {
        name = "${name}-binaries";
        paths = map (binary: jail binary "${app.package}/bin/${binary}" permissions) app.binaries;
      };
    in
    pkgs.symlinkJoin {
      name = "${name}-jailed";
      paths = [
        binaries
        (resources name app binaries)
      ];
      passthru = {
        inherit binaries;
        unjailed = app.package;
        storage = storageOf name app;
      };
      # `outputsToInstall` would follow the original into a join that has only
      # one output, and the environment build then fails looking for `man`.
      meta = removeAttrs app.package.meta [ "outputsToInstall" ] // {
        mainProgram = lib.head app.binaries;
      };
    };

  fileModule = {
    options = {
      source = mkOption {
        type = types.nullOr types.path;
        default = null;
        description = "Contents taken from an existing path.";
      };

      text = mkOption {
        type = types.nullOr types.lines;
        default = null;
        description = "Contents given literally.";
      };

      generator = mkOption {
        type = types.functionTo types.raw;
        default = lib.id;
        description = "Applied to `value` to produce the contents.";
      };

      value = mkOption {
        type = types.raw;
        default = null;
        description = "Structured contents, rendered by `generator`.";
      };

      mutable = mkOption {
        type = types.bool;
        default = true;
        description = ''
          Whether the app is allowed to write this file. A mutable file is
          copied into the private home before each launch, so a write can
          never fail and the declared value is restored the next time the app
          starts. An immutable file is bind mounted read-only instead, which
          leaves nothing behind but makes any write fail outright.
        '';
      };

      mode = mkOption {
        type = types.str;
        default = "644";
        description = "Permissions of the seeded copy; only meaningful when mutable.";
      };
    };
  };

  appModule =
    { name, config, ... }:
    {
      options = {
        package = mkOption {
          type = types.package;
          description = "The application itself.";
        };

        binaries = mkOption {
          type = types.listOf types.str;
          default = [ name ];
          description = ''
            Executables to wrap, when they differ from the attribute name or
            the package ships more than one.
          '';
        };

        appId = mkOption {
          type = types.nullOr types.str;
          default = null;
          description = ''
            Reverse-DNS identifier presented to xdg-desktop-portal. Setting it
            grants the app portal access and its own namespace in the document
            portal; leaving it null withholds both.
          '';
        };

        backup = mkOption {
          type = types.bool;
          default = false;
          description = ''
            Whether the app's private home belongs on /persist, and so in the
            snapshot timeline, rather than on /cache.
          '';
        };

        files = mkOption {
          type = types.attrsOf (types.submodule fileModule);
          default = { };
          description = "Configuration placed in the app's private home, keyed by path below it.";
        };

        access = mkOption {
          type = types.attrsOf (
            types.enum [
              "ro"
              "rw"
            ]
          );
          default = { };
          example = {
            download = "rw";
            Seafile = "rw";
          };
          description = ''
            What the application may reach in the shared home, and how. A key
            is either the lowercased middle of an XDG_*_DIR variable or a path
            below the home directory. Prefer withholding these and letting the
            application go through the file chooser portal instead.
          '';
        };

        directories = mkOption {
          type = types.listOf types.str;
          default = [ ];
          description = ''
            Directories to make in the private home, for the ones an app
            expects to find rather than create.
          '';
        };

        links = mkOption {
          type = types.attrsOf types.str;
          default = { };
          example = {
            ".local/share/app/instance/save" = ".local/share/app/saves/one";
          };
          description = ''
            Symbolic links made inside the private home, from path to target,
            both relative to it.
          '';
        };

        jail.permissions = mkOption {
          type = types.functionTo (types.listOf types.raw);
          default = _: [ ];
          example = lib.literalExpression "c: with c; [ electron network notifications ]";
          description = "Extra jail.nix combinators, on top of those derived from the declarations above.";
        };

        wrapped = mkOption {
          type = types.package;
          readOnly = true;
          description = "The package as it is actually installed.";
        };

        storage = mkOption {
          type = types.str;
          readOnly = true;
          description = "Where the private home lives on the host.";
        };
      };

      config = {
        wrapped = wrap name config;
        storage = storageOf name config;
      };
    };

  apps = attrValues cfg;
in
{
  options.apps = mkOption {
    type = types.attrsOf (types.submodule appModule);
    default = { };
    description = ''
      Sandboxed applications, declared in one place: what to install, what
      configuration they get, and what they are allowed to reach. Their state
      lives in a private home rather than in the user's, so none of it goes
      through preservation.
    '';
  };

  config.packages = map (app: app.wrapped) apps;
}
