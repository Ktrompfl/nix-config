{ inputs, pkgs, ... }:
inputs.jail.lib.extend {
  inherit pkgs;

  additionalCombinators =
    c: with c; rec {
      # xdg-desktop-portal identifies its caller by reading
      # `/proc/<pid>/root/.flatpak-info`, so a jail that wants portals has to
      # present one. With an app id the document portal namespaces the files
      # handed to this app under `by-app/<id>`, which is bound over the jail's
      # view of the document store so that it can see those and nothing else.
      portals =
        appId:
        compose [
          (dbus {
            talk = [
              "org.freedesktop.portal.*"
              "org.freedesktop.DBus"
              # gtk looks this up on every start and complains loudly when it
              # cannot reach it
              "org.a11y.Bus"
            ];
            # GApplication and friends register themselves under their own
            # application id at startup and refuse to run when they cannot.
            own = [ appId ];
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
        ];

      # What any windowed application needs; the toolkit environment comes
      # from the module, which forwards it for every app.
      desktop = compose [
        gui
        gpu
      ];

      # StatusNotifierItem, which wl-tray-bridge watches for on jay's behalf.
      #
      # Qt registers itself as `org.kde.StatusNotifierItem-<pid>-<n>`, and
      # xdg-dbus-proxy only wildcards on a dot boundary, so there is no way to
      # name that family more narrowly than the whole `org.kde` prefix. Note
      # that an invalid name here does not fail the build: the proxy exits,
      # never writes to the ready fifo, and the wrapper blocks forever.
      tray = dbus {
        talk = [ "org.kde.StatusNotifierWatcher" ];
        own = [ "org.kde.*" ];
      };

      # Media keys and the bar's now-playing segment go through this.
      mpris = name: dbus { own = [ "org.mpris.MediaPlayer2.${name}" ]; };

      # Direct control of the sound hardware, for the mixer interfaces. They
      # find the card by walking sysfs for its usb device, so the control
      # nodes alone are not enough.
      sound = compose [
        (unsafe-add-raw-args "--dev-bind-try /dev/snd /dev/snd")
        (unsafe-add-raw-args "--ro-bind-try /sys/bus/usb /sys/bus/usb")
        (unsafe-add-raw-args "--ro-bind-try /sys/devices /sys/devices")
      ];

      # `readonly-paths-from-var` binds each entry at its *resolved* path, so a
      # variable naming a profile directory leaves the program looking at a
      # path the jail does not have: the theme plugins and icon themes in
      # ~/.nix-profile or /etc/profiles are simply absent. Bind the resolved
      # content at the name the variable actually gives instead.
      paths-from-var =
        variable: separator:
        add-runtime ''
          IFS='${separator}' read -ra VAR_ENTRIES <<< "''${${variable}-}"
          for VAR_ENTRY in ''${VAR_ENTRIES+"''${VAR_ENTRIES[@]}"}; do
            if [ -n "$VAR_ENTRY" ] && [ -e "$VAR_ENTRY" ]; then
              RUNTIME_ARGS+=(--ro-bind "$(realpath -- "$VAR_ENTRY")" "$VAR_ENTRY")
            fi
          done
        '';

      # glibc finds its locales through this and falls back to the C locale,
      # noisily, without it.
      locale = compose [
        (try-fwd-env "LOCALE_ARCHIVE")
        (add-runtime ''
          if [ -n "''${LOCALE_ARCHIVE-}" ] && [ -e "$LOCALE_ARCHIVE" ]; then
            RUNTIME_ARGS+=(--ro-bind "$(realpath -- "$LOCALE_ARCHIVE")" "$LOCALE_ARCHIVE")
          fi
        '')
      ];

      # Viewers are handed a path on the command line, so bind whatever they
      # were pointed at rather than granting a whole directory in advance.
      #
      # `readonly-runtime-args` binds each argument at its *resolved* path,
      # which is not where the application looks: an argument below the home
      # directory resolves onto the preservation storage, while the program is
      # still handed the path it was given, which lands in the private home.
      # So bind the resolved file at the absolute form of the argument, and
      # hand the program that absolute form -- which also makes relative
      # arguments work, since the jail has no useful working directory.
      open-args = compose [
        (add-runtime ''
          JAIL_ARGV=()
          for ARGUMENT in "$@"; do
            if [ -e "$ARGUMENT" ]; then
              # Made absolute against the logical working directory rather
              # than with realpath, which would resolve the preservation
              # symlinks and hand the program a /persist or /cache path to
              # record in its own state.
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

      viewer = compose [
        desktop
        open-args
      ];
    };
}
