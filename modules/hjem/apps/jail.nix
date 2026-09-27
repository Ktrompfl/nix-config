{ inputs, pkgs, ... }:
inputs.jail.lib.extend {
  inherit pkgs;

  # Bind the whole store instead of each app's closure: jail.nix computes that
  # at every launch, which takes most of a second, and the store is
  # world-readable anyway. The stub turns jail.nix's closure walk into a no-op;
  # the bogus `runtime-deep-ro-bind` makes jail.nix define it first, so the
  # stub can replace it.
  basePermissions =
    c: with c; [
      base
      fake-passwd

      (unsafe-add-raw-args "--ro-bind /nix/store /nix/store")
      (runtime-deep-ro-bind "/nonexistent-emits-the-helpers")
      (add-runtime "bindNixStoreClosure() { :; }")
    ];

  additionalCombinators =
    c: with c; rec {
      # xdg-desktop-portal identifies the app by its `/.flatpak-info`. Only the
      # files handed to this app are visible from the document portal.
      portals =
        appId:
        compose [
          (dbus {
            talk = [
              "org.freedesktop.portal.*"
              "org.freedesktop.DBus"
              # gtk warns without it
              "org.a11y.Bus"

              # a second instance hands over to the first through these
              "${appId}.*"
            ];
            # GApplication refuses to start without its app id; Firefox and
            # Thunderbird own a name below it per profile.
            own = [
              appId
              "${appId}.*"
            ];
          })

          (write-text "/.flatpak-info" ''
            [Application]
            name=${appId}

            [Instance]
            instance-id=jail
            session-bus-proxy=true
            system-bus-proxy=false
          '')

          (fwd-env "XDG_RUNTIME_DIR")
          (rw-bind (noescape "\"$XDG_RUNTIME_DIR/doc/by-app/${appId}\"") (
            noescape "\"$XDG_RUNTIME_DIR/doc\""
          ))

          open-uri
        ];

      # An `xdg-open` that forwards to the portal, so links open on the host;
      # toolkits call `xdg-open`, and no handler exists in the jail.
      open-uri = add-pkg-deps [
        (pkgs.writeShellApplication {
          name = "xdg-open";
          runtimeInputs = [ pkgs.glib ];
          text = ''
            for target in "$@"; do
              case "$target" in
                *://*) uri="$target" ;;
                /*) uri="file://$target" ;;
                *) uri="file://$PWD/$target" ;;
              esac

              gdbus call --session \
                --dest org.freedesktop.portal.Desktop \
                --object-path /org/freedesktop/portal/desktop \
                --method org.freedesktop.portal.OpenURI.OpenURI \
                "" "$uri" "{}" >/dev/null
            done
          '';
        })
      ];

      # Tray icons. Qt owns `org.kde.StatusNotifierItem-<pid>-<n>`, which
      # xdg-dbus-proxy can only match as `org.kde.*`. An invalid name here
      # makes the wrapper hang rather than fail.
      tray = dbus {
        talk = [ "org.kde.StatusNotifierWatcher" ];
        own = [ "org.kde.*" ];
      };

      # Media keys and now-playing.
      mpris = name: dbus { own = [ "org.mpris.MediaPlayer2.${name}" ]; };

      # Sound hardware, for mixers. They find the card through its usb device
      # in sysfs.
      sound = compose [
        (unsafe-add-raw-args "--dev-bind-try /dev/snd /dev/snd")
        (unsafe-add-raw-args "--ro-bind-try /sys/bus/usb /sys/bus/usb")
        (unsafe-add-raw-args "--ro-bind-try /sys/devices /sys/devices")
      ];

      # Bind the files given as arguments read-only and pass them as absolute
      # paths. Unlike `readonly-runtime-args`, which binds at the resolved
      # path, this binds at the path the app is given.
      open-args = compose [
        (add-runtime ''
          JAIL_ARGV=()
          for ARGUMENT in "$@"; do
            if [ -e "$ARGUMENT" ]; then
              # not realpath, which would expose /persist or /cache paths
              case "$ARGUMENT" in
                /*) ABSOLUTE="$ARGUMENT" ;;
                *) ABSOLUTE="$PWD/$ARGUMENT" ;;
              esac
              RUNTIME_ARGS+=(--ro-bind "$(realpath -- "$ARGUMENT")" "$ABSOLUTE")
              JAIL_ARGV+=("$ABSOLUTE")
            else
              JAIL_ARGV+=("$ARGUMENT")
            fi
          done
        '')
        (set-argv [ (noescape ''"''${JAIL_ARGV[@]}"'') ])
      ];
    };
}
