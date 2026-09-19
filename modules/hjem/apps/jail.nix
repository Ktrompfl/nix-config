{ inputs, pkgs, ... }:
inputs.jail.lib.extend {
  inherit pkgs;

  # jail.nix binds the runtime closure of everything it puts in the jail,
  # shelling out to `nix-store --query --requisites` once per store symlink it
  # walks. That was eight invocations at roughly 0.13s each -- most of a
  # second added to every single launch -- to reconstruct something the store
  # already is. Bind the store instead: it is world-readable on the host, so
  # this gives away nothing the threat model cares about, and it makes every
  # one of those queries redundant.
  #
  # The stub below leans on jail.nix internals: `runtime-deep-ro-bind` defines
  # its shell helpers the first time it is composed, so calling it here on a
  # path that cannot exist emits them without doing any work, and the closure
  # walk can then be replaced before `gui`, `gpu` and `network` reach for it.
  # If a future jail.nix renames that helper this stops taking effect and the
  # launches get slow again -- it cannot break correctness, only speed.
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

              # a second instance reaches the first under a name below the
              # application id, so it has to be able to call it as well
              "${appId}.*"
            ];
            # GApplication and friends register themselves under their own
            # application id at startup and refuse to run when they cannot.
            # Firefox and Thunderbird additionally own a name below it, one
            # per profile, and that is how a second invocation hands a url to
            # the instance that is already running; without it every `firefox
            # <url>` becomes a fresh process that finds the profile locked and
            # gives up.
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

      # Electron and most toolkits open an external link by shelling out to
      # `xdg-open`, which a jail has no reason to contain -- and the real one
      # would be no use anyway, since it would look for a handler that is not
      # in here either. Flatpak answers this with an `xdg-open` that forwards
      # to the portal instead; do the same, so the host decides what opens the
      # link and the jail never needs to see a browser.
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
