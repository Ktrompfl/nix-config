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
    concatMapStringsSep
    elem
    escapeShellArg
    filterAttrs
    hasPrefix
    isDerivation
    isPath
    mapAttrsToList
    mkOption
    optional
    types
    unique
    ;

  jail = import ./jail.nix { inherit inputs pkgs; };

  home = config.directory;

  # The app's private home, bound over $HOME in the jail. It lives below the
  # storage roots so that the user can create it without tmpfiles.
  storageOf =
    name: app: "${if app.backup then config.storage.data else config.storage.state}/apps/${name}";

  # An `access` key: an XDG user directory by name, or else a path below $HOME.
  grantedPath =
    key: config.environment.sessionVariables."XDG_${lib.toUpper key}_DIR" or "${home}/${key}";

  # Toolkit and cursor theming from the shared home, bound from the store so
  # that the jail does not see the preservation symlinks.
  themingFiles =
    let
      from =
        prefix: files:
        mapAttrsToList (path: file: {
          path = "${prefix}/${path}";
          inherit (file) source;
        }) files;
      isTheming =
        path:
        any (directory: hasPrefix "${directory}/" path) [
          "gtk-3.0"
          "gtk-4.0"
          "Kvantum"
          "qt5ct"
          "qt6ct"
        ];
    in
    from ".config" (filterAttrs (path: _: isTheming path) config.xdg.config.files)
    ++ from ".local/share" (
      filterAttrs (path: _: elem path [ "icons/default/index.theme" ]) config.xdg.data.files
    );

  # The same file options as hjem's, so that a file can move between
  # `xdg.config.files` and `apps.<name>.files` unchanged.
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

  # Relative, so that links in the private home also resolve on the host.
  relativeTo =
    from: to: lib.concatStrings (map (_: "../") (lib.init (lib.splitString "/" from))) + to;

  permissionsOf =
    name: app:
    let
      storage = storageOf name app;
      inStorage = path: escapeShellArg "${storage}/${path}";

      copied = filterAttrs (_: file: file.mutable) app.files;
      bound =
        mapAttrsToList (path: file: {
          inherit path;
          source = contentsOf path file;
        }) (filterAttrs (_: file: !file.mutable) app.files)
        ++ themingFiles;

      # What was copied in last time, so that removed files can be deleted.
      manifest = pkgs.writeText "${name}-managed" (concatLines (attrNames copied));

      # bwrap needs the parent of every bind destination to exist.
      parents = unique (
        map (file: dirOf file.path) bound
        ++ mapAttrsToList (path: _: dirOf path) app.links
        ++ app.directories
      );
    in
    c:
    with c;
    [
      # Prepares the private home on the host before each launch.
      (add-runtime ''
        APP_HOME=${escapeShellArg storage}
        mkdir -p "$APP_HOME" ${concatMapStringsSep " " inStorage parents}

        if [ -f "$APP_HOME/.managed" ]; then
          comm -23 <(sort "$APP_HOME/.managed") <(sort ${manifest}) | while IFS= read -r stale; do
            [ -n "$stale" ] && rm -f "$APP_HOME/$stale"
          done
        fi
        install -D -m644 ${manifest} "$APP_HOME/.managed"

        # bwrap cannot bind over a symlink that leads out of the jail
        rm -f ${concatMapStringsSep " " (file: inStorage file.path) bound}

        ${concatLines (
          mapAttrsToList (
            path: target: "ln -sfn ${escapeShellArg (relativeTo path target)} ${inStorage path}"
          ) app.links
        )}

        # Copied rather than bound, so that the app can write them however it
        # likes; the declared contents return at the next launch. `-p` keeps
        # the mtime, which apps use to cache work on the file.
        ${concatLines (
          mapAttrsToList (
            path: file: "install -D -p -m644 ${escapeShellArg "${contentsOf path file}"} ${inStorage path}"
          ) copied
        )}
      '')

      (rw-bind storage home)

      # The profiles that the variables below point into.
      (readonly "/run/current-system")
      (readonly "/etc/profiles/per-user/${config.user}")
    ]

    # The jail clears the environment; these are needed for theming, the
    # Wayland backends and the locale.
    ++ map try-fwd-env [
      "GTK2_RC_FILES"
      "GTK_A11Y"
      "GTK_PATH"
      "GTK_THEME"
      "LOCALE_ARCHIVE"
      "NIXOS_OZONE_WL"
      "QT_PLUGIN_PATH"
      "QT_QPA_PLATFORM"
      "QT_QPA_PLATFORMTHEME"
      "QT_STYLE_OVERRIDE"
      "XDG_CURRENT_DESKTOP"
      "XENVIRONMENT"
    ]

    ++ map (file: ro-bind file.source "${home}/${file.path}") bound

    ++ mapAttrsToList (
      key: mode: (if mode == "rw" then readwrite else readonly) (grantedPath key)
    ) app.access

    ++ optional (app.appId != null) (portals app.appId)

    ++ app.jail.permissions c;

  # The package's share/, with the desktop entries pointed at the jailed
  # binaries, so that launchers and `xdg-open` do not run the unjailed ones.
  resources =
    name: app: binaries:
    pkgs.runCommand "${name}-resources" { } ''
      ${lib.optionalString (app.appId != null) ''
        # The jail only lets the app own `appId` on the bus; a mismatch with its
        # desktop entry makes it exit at startup, so fail the build instead.
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
                concatMapStringsSep " " (
                  binary: "-e 's|${app.package}/bin/${binary}|${binaries}/bin/${binary}|g'"
                ) app.binaries
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
      # `outputsToInstall` names outputs that the join does not have.
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
          Whether the app may write the file. A mutable file is copied in at
          each launch, an immutable one is bound read-only.
        '';
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
          description = "Executables to wrap.";
        };

        appId = mkOption {
          type = types.nullOr types.str;
          default = null;
          description = "Application id; setting it grants access to the portals.";
        };

        backup = mkOption {
          type = types.bool;
          default = false;
          description = "Whether the private home is on /persist rather than /cache.";
        };

        files = mkOption {
          type = types.attrsOf (types.submodule fileModule);
          default = { };
          description = "Files in the private home, keyed by path below it.";
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
            Paths in the shared home the app may reach: an XDG user directory by
            its lowercased name, or a path below the home directory.
          '';
        };

        directories = mkOption {
          type = types.listOf types.str;
          default = [ ];
          description = "Directories to create in the private home.";
        };

        links = mkOption {
          type = types.attrsOf types.str;
          default = { };
          example = {
            ".local/share/app/instance/save" = ".local/share/app/saves/one";
          };
          description = "Symlinks in the private home, from path to target, both relative to it.";
        };

        jail.permissions = mkOption {
          type = types.functionTo (types.listOf types.raw);
          default = _: [ ];
          example = lib.literalExpression "c: with c; [ electron network notifications ]";
          description = "Additional jail.nix combinators.";
        };

        wrapped = mkOption {
          type = types.package;
          readOnly = true;
          description = "The jailed package, as installed.";
        };

        storage = mkOption {
          type = types.str;
          readOnly = true;
          description = "The private home on the host.";
        };
      };

      config = {
        wrapped = wrap name config;
        storage = storageOf name config;
      };
    };
in
{
  options.apps = mkOption {
    type = types.attrsOf (types.submodule appModule);
    default = { };
    description = "Applications that run in a jail with a private home.";
  };

  config.packages = map (app: app.wrapped) (attrValues config.apps);
}
