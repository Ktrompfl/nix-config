{
  lib,
  pkgs,
  ...
}:
let
  colorVars = lib.concatStringsSep "\n" [
    "BLUE=$'\\033[34m'" # directory
    "GREEN=$'\\033[32m'" # clean git / low usage
    "RED=$'\\033[31m'" # dirty git / high usage
    "YELLOW=$'\\033[33m'" # mid usage
    "MAGENTA=$'\\033[35m'" # model
    "CYAN=$'\\033[36m'" # output style
    "MUTED=$'\\033[90m'" # separators
    "RESET=$'\\033[0m'"
  ];

  statuslineScript = pkgs.writeShellApplication {
    name = "claude-statusline";
    runtimeInputs = [
      pkgs.jq
      pkgs.git
    ];
    text = ''
      ${colorVars}

      input=$(cat)

      cwd=$(jq -r '.workspace.current_dir // .cwd // empty' <<<"$input")
      [ -z "$cwd" ] && cwd="$PWD"
      display_dir=''${cwd/#$HOME/\~}

      model=$(jq -r '.model.display_name // empty' <<<"$input")
      style=$(jq -r '.output_style.name // empty' <<<"$input")
      used_pct=$(jq -r '.context_window.used_percentage // empty' <<<"$input")
      five=$(jq -r '.rate_limits.five_hour.used_percentage // empty' <<<"$input")
      week=$(jq -r '.rate_limits.seven_day.used_percentage // empty' <<<"$input")

      pct_color() {
        local pct=''${1%.*}
        if [ "''${pct:-0}" -ge 80 ]; then
          printf '%s' "$RED"
        elif [ "''${pct:-0}" -ge 50 ]; then
          printf '%s' "$YELLOW"
        else
          printf '%s' "$GREEN"
        fi
      }

      git_segment=""
      if git -C "$cwd" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
        branch=$(git -C "$cwd" branch --show-current 2>/dev/null)
        [ -z "$branch" ] && branch=$(git -C "$cwd" rev-parse --short HEAD 2>/dev/null)
        dirty=""
        branch_color="$GREEN"
        if [ -n "$(git -C "$cwd" status --porcelain 2>/dev/null)" ]; then
          dirty="*"
          branch_color="$RED"
        fi
        [ -n "$branch" ] && git_segment=" ''${branch_color}''${branch}''${dirty}''${RESET}"
      fi

      line="''${BLUE}''${display_dir}''${RESET}''${git_segment}"

      if [ -n "$model" ]; then
        line="''${line} ''${MAGENTA}[''${model}]''${RESET}"
      fi

      if [ -n "$style" ] && [ "$style" != "default" ]; then
        line="''${line} ''${CYAN}(''${style})''${RESET}"
      fi

      if [ -n "$used_pct" ]; then
        c=$(pct_color "$used_pct")
        line="''${line} ''${MUTED}|''${RESET} ''${c}ctx ''${used_pct%.*}%''${RESET}"
      fi

      rl_segment=""
      if [ -n "$five" ]; then
        c=$(pct_color "$five")
        rl_segment="''${c}5h ''${five%.*}%''${RESET}"
      fi
      if [ -n "$week" ]; then
        c=$(pct_color "$week")
        [ -n "$rl_segment" ] && rl_segment="''${rl_segment} "
        rl_segment="''${rl_segment}''${c}7d ''${week%.*}%''${RESET}"
      fi
      [ -n "$rl_segment" ] && line="''${line} ''${MUTED}|''${RESET} ''${rl_segment}"

      printf '%s' "$line"
    '';
  };
in
{
  programs.claude-code.settings.statusLine = {
    type = "command";
    command = lib.getExe statuslineScript;
    padding = 0;
  };
}
