{
  dock = "right";
  show_turn_stats = true;
  sidebar_side = "right";

  tool_permissions = {
    default = "confirm";
    tools = {
      search_web.default = "allow";
      fetch = {
        default = "confirm";
        always_allow = [
          { pattern = "^https://(github\\.com|raw\\.githubusercontent\\.com|devenv\\.sh)/"; }
          { "pattern" = "docs\\.rs"; }
        ];
      };

      create_directory.default = "allow";
      copy_path.default = "confirm";
      move_path.default = "confirm";
      delete_path = {
        default = "confirm";
        always_deny = [
          { pattern = "^/$"; }
        ];
      };

      edit_file = {
        default = "confirm";
        # protect sensitive files
        always_deny = [
          { pattern = "\\.env"; }
          { pattern = "secrets?/"; }
          { pattern = "\\.(pem|key)$"; }
        ];
      };
      write_file.default = "confirm";

      terminal = {
        default = "confirm";
        always_allow = [
          # Read-only git commands
          {
            pattern = "^git (status|log|diff|show|branch|remote|blame|ls-files|rev-parse|describe|shortlog|reflog|cat-file|grep|ls-tree|show-ref|for-each-ref|rev-list|merge-base|name-rev)\\b";
          }
          {
            pattern = "^git (stash list|worktree list|submodule status|config (--get|--list|-l))\\b";
          }

          # Safe file system operations
          {
            pattern = "^(ls|find|fd|cat|head|tail|pwd|stat|file|wc|tree|realpath|readlink|dirname|basename|du|df)\\b";
          }

          # Safe read-only text/data inspection
          {
            pattern = "^(rg|grep|diff|sort|uniq|cut|comm|column|jq|nl|tac|rev|tr)\\b";
          }
          { pattern = "^sed -n\\b"; }

          # Safe read-only binary/hash inspection
          {
            pattern = "^(od|xxd|hexdump|strings|base64|cksum|md5sum|sha1sum|sha256sum|sha512sum|b2sum)\\b";
          }

          # Safe read-only system info
          {
            pattern = "^(whoami|id|hostname|uname|date|uptime|env|printenv|which|type|getconf|free|ps|pgrep|lsof|ss|lscpu|lsblk|lsusb|lspci|findmnt|getent|groups|locale)\\b";
          }
          { pattern = "^command -v\\b"; }

          # Safe nix read operations
          {
            pattern = "^nix (eval|flake (show|metadata|check)|search|log|path-info|derivation show|why-depends|store (ls|cat|info)|config show|show-config|registry list|profile list)\\b";
          }
          { pattern = "^nix-instantiate --parse\\b"; }
          { pattern = "^nix-store (-q|--query)\\b"; }
          { pattern = "^(nixos-option|statix check|nh search)\\b"; }

          # Cargo operations
          { pattern = "^cargo\\s+(check|build|test|clippy|fmt|doc)\\b"; }

          # Jujutsu read-only

          # GitHub CLI read-only; gh api stays on confirm since it can mutate.
          {
            pattern = "^gh (pr (view|list|diff|checks|status)|issue (view|list|status)|run (list|view)|repo view|release (list|view)|label list|search)\\b";
          }

          # Git staging, directory creation, misc read-only system info
          { pattern = "^git add\\b"; }
          { pattern = "^(mkdir|chmod)\\b"; }
          { pattern = "^systemctl (list-units|list-timers|status)\\b"; }
          { pattern = "^(journalctl|dmesg)\\b"; }
          { pattern = "^claude --version$"; }
          { pattern = "^coredumpctl list\\b"; }
        ];
        always_confirm = [
          # Potentially destructive git commands
          { pattern = "^git (checkout|commit|merge|pull|push|rebase|reset|restore|switch)\\b"; }
          { pattern = "^git stash($|\\s+(push|pop|apply|drop|clear|store|branch))"; }

          # File deletion and modification
          { pattern = "^(cp|mv|rm|dd|mkfs|shutdown|reboot)\\b"; }

          # System control operations
          { pattern = "^systemctl (disable|enable|mask|reload|restart|start|stop|unmask)\\b"; }

          # Network operations
          { pattern = "^(curl|ping|rsync|scp|ssh|wget)\\b"; }

          # Package management
          { pattern = "^nix (build|develop|run|shell)\\b"; }
          { pattern = "^(nixos-rebuild|sudo)\\b"; }

          # Process management
          { pattern = "^(kill|killall|pkill)\\b"; }
        ];
        always_deny = [
          { pattern = "^rm -rf /(\\*)?$"; }
        ];
      };
    };
  };
}
