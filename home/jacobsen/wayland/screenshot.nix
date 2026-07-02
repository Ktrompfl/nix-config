{ lib, pkgs, ... }: {
  home.packages = [
    # TODO: when this is implemented, allow to screenshot the focused window / workspace / output
    # E501: ignore line length violations
    (pkgs.writers.writePython3Bin "screenshot"
      {
        doCheck = false;
        flakeIgnore = [ "E501" ];
      }
      /* py */ ''
        import argparse
        import os
        import re
        import shutil
        import subprocess
        import sys
        import tempfile
        from datetime import datetime
        from pathlib import Path

        GRIM = "${lib.getExe pkgs.grim}"
        SLURP = "${lib.getExe pkgs.slurp}"
        SATTY = "${lib.getExe pkgs.satty}"
        WL_COPY = "${lib.getExe' pkgs.wl-clipboard "wl-copy"}"
        NOTIFY_SEND = "${lib.getExe' pkgs.libnotify "notify-send"}"
        JAY = "jay"

        DEFAULT_OUTPUT_FORMAT = "%Y-%m-%d-%H%M%S.png"

        REGION_RE = re.compile(r"^-?\d+,-?\d+ \d+x\d+$")
        POS_RE = re.compile(r"pos:\s*(-?\d+)x(-?\d+)\+(-?\d+)x(-?\d+)")


        class ScreenshotError(Exception):
            """Raised for any expected/handled failure.

            The message is shown directly to the user.
            """


        def run(cmd, **kwargs):
            """Run a subprocess.

            Raises ScreenshotError with a clear message on failure.
            """
            try:
                return subprocess.run(cmd, check=True, **kwargs)
            except FileNotFoundError as e:
                raise ScreenshotError(
                    f"command not found: {cmd[0]} ({e})"
                ) from e
            except subprocess.CalledProcessError as e:
                stderr = ""
                if e.stderr:
                    stderr = e.stderr.decode(errors="replace").strip()
                detail = f": {stderr}" if stderr else ""
                raise ScreenshotError(
                    f"command failed ({' '.join(cmd)}), "
                    f"exit code {e.returncode}{detail}"
                ) from e


        def region_from_jay_query(query: str) -> str:
            """Query jay for a window/workspace position.

            Converts the result to grim/slurp 'X,Y WxH' geometry format.
            """
            result = run(
                [JAY, "tree", "query", query],
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )
            output = result.stdout.decode(errors="replace")

            match = None
            for line in output.splitlines():
                m = POS_RE.search(line.replace(" ", ""))
                if m:
                    match = m
                    break

            if match is None:
                raise ScreenshotError(
                    f"could not find a 'pos:' entry in "
                    f"'{JAY} tree query {query}' output"
                )

            x, y, w, h = match.groups()
            return f"{x},{y} {w}x{h}"


        def region_from_slurp() -> str:
            """Prompt the user to select a region via slurp."""
            try:
                result = run(
                    [SLURP], stdout=subprocess.PIPE, stderr=subprocess.PIPE
                )
            except ScreenshotError as e:
                # slurp exits non-zero if cancelled (e.g. Escape).
                raise ScreenshotError(
                    f"region selection cancelled or failed ({e})"
                ) from e
            region = result.stdout.decode(errors="replace").strip()
            if not region:
                raise ScreenshotError(
                    "region selection cancelled (empty selection)"
                )
            return region


        def get_region(mode: str) -> str:
            if mode == "workspace":
                region = region_from_jay_query("select-workspace")
            elif mode == "window":
                region = region_from_jay_query("select-window")
            elif mode == "region":
                region = region_from_slurp()
            else:
                raise ScreenshotError(f"unknown mode: {mode}")

            if not REGION_RE.match(region):
                raise ScreenshotError(f"invalid region geometry: '{region}'")
            return region


        def capture(region: str, dest: Path) -> None:
            run([GRIM, "-g", region, str(dest)], stderr=subprocess.PIPE)
            if not dest.exists() or dest.stat().st_size == 0:
                raise ScreenshotError("grim produced no output")


        def edit_with_satty(src: Path, dest: Path) -> None:
            run(
                [
                    SATTY,
                    "--filename",
                    str(src),
                    "--output-filename",
                    str(dest),
                ],
                stderr=subprocess.PIPE,
            )
            if not dest.exists() or dest.stat().st_size == 0:
                raise ScreenshotError(
                    "satty produced no output (editing cancelled?)"
                )


        def save(src: Path, path_format: str) -> Path:
            try:
                expanded = datetime.now().strftime(path_format)
            except ValueError as e:
                raise ScreenshotError(
                    f"invalid --output path format '{path_format}': {e}"
                ) from e

            dest = Path(expanded).expanduser()
            try:
                dest.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(src, dest)
            except OSError as e:
                raise ScreenshotError(
                    f"failed to write output file '{dest}': {e}"
                ) from e
            return dest


        def copy_to_clipboard(src: Path) -> None:
            try:
                data = src.read_bytes()
            except OSError as e:
                raise ScreenshotError(
                    f"failed to read '{src}' for clipboard copy: {e}"
                ) from e
            run(
                [WL_COPY, "--type", "image/png"],
                input=data,
                stderr=subprocess.PIPE,
            )


        def notify(title: str, body: str, icon: Path | None) -> None:
            cmd = [
                NOTIFY_SEND,
                "--app-name=Screenshot",
                "--urgency=normal",
                "--expire-time=3000",
            ]
            if icon is not None:
                cmd.append(f"--icon={icon}")
            cmd += [title, body]
            # Notification failures shouldn't abort a successful screenshot.
            try:
                run(cmd, stderr=subprocess.PIPE)
            except ScreenshotError as e:
                print(f"warning: notification failed: {e}", file=sys.stderr)


        def default_output_path() -> str:
            base = os.environ.get("XDG_SCREENSHOTS_DIR")
            if not base:
                base = str(Path.home() / "Pictures" / "screenshots")
            return str(Path(base) / DEFAULT_OUTPUT_FORMAT)


        def parse_args(argv):
            parser = argparse.ArgumentParser(
                prog="screenshot",
                description=(
                    "Take a screenshot of a window, region, or workspace."
                ),
            )
            parser.add_argument(
                "mode",
                choices=["window", "region", "workspace"],
                help="how to select the screenshot area",
            )
            parser.add_argument(
                "--copy",
                action="store_true",
                help="copy the result to the clipboard",
            )
            parser.add_argument(
                "--notify",
                action="store_true",
                help="send a desktop notification",
            )
            parser.add_argument(
                "--satty",
                action="store_true",
                help="open the capture in satty before saving/copying",
            )
            parser.add_argument(
                "--output",
                nargs="?",
                const="",
                default=None,
                metavar="PATH",
                help=(
                    "save to PATH (may contain strftime placeholders, "
                    "e.g. %%Y-%%m-%%d.png). If PATH is omitted, defaults "
                    "to $XDG_SCREENSHOTS_DIR/"
                    + DEFAULT_OUTPUT_FORMAT
                    + " or ~/Pictures/screenshots/"
                    + DEFAULT_OUTPUT_FORMAT
                ),
            )
            args = parser.parse_args(argv)

            if args.output == "":
                args.output = default_output_path()

            return args


        def main(argv=None) -> int:
            args = parse_args(argv)

            tmpdir = Path(tempfile.mkdtemp(prefix="screenshot-"))
            try:
                region = get_region(args.mode)

                capture_path = tmpdir / "capture.png"
                capture(region, capture_path)

                final_path = capture_path
                if args.satty:
                    edited_path = tmpdir / "edited.png"
                    edit_with_satty(capture_path, edited_path)
                    final_path = edited_path

                saved_path = None
                if args.output is not None:
                    saved_path = save(final_path, args.output)

                if args.copy:
                    copy_to_clipboard(final_path)

                if args.notify:
                    if saved_path is not None:
                        notify(
                            "Screenshot Saved",
                            f"Screenshot saved to {saved_path}",
                            saved_path,
                        )
                    else:
                        notify(
                            "Screenshot Taken",
                            "Screenshot captured",
                            final_path,
                        )

                no_action = not (
                    args.copy or args.notify or args.output is not None
                )
                if no_action:
                    print(
                        "warning: no action requested (use --copy, "
                        "--notify, and/or --output); screenshot was "
                        "taken but discarded",
                        file=sys.stderr,
                    )

                return 0

            except ScreenshotError as e:
                print(f"error: {e}", file=sys.stderr)
                return 1
            except KeyboardInterrupt:
                print("error: interrupted", file=sys.stderr)
                return 130
            finally:
                shutil.rmtree(tmpdir, ignore_errors=True)


        if __name__ == "__main__":
            sys.exit(main())
      ''
    )
  ];
}
