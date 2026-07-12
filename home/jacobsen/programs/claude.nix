{
  config,
  inputs,
  lib,
  pkgs,
  ...
}:
let
  inherit (lib)
    getExe
    getExe'
    genAttrs
    ;

  toLang = lang: exts: genAttrs exts (_: lang);
in
{
  # TODO: maybe install https://mcp-nixos.io/
  programs.claude-code = {
    enable = true;
    package = inputs.llm-agents.packages.${pkgs.stdenv.hostPlatform.system}.claude-code;
    settings = {
      permissions = {
        allow = [
          # Core Claude Code tools
          "Glob(*)"
          "Grep(*)"
          "LS(*)"
          "Read(*)"
          "Search(*)"
          "Task(*)"
          "TodoWrite(*)"
          "WebSearch"

          # Skill, command, and agent references live outside the project root
          # (the Claude configDir plus Nix store symlinks), so Read(*) - which is
          # scoped to the workspace - does not cover them.
          "Read(${config.home.homeDirectory}/.claude/**)"
          "Read(/nix/store/**)"

          # Safe read-only git commands
          "Bash(git status)"
          "Bash(git status:*)"
          "Bash(git log:*)"
          "Bash(git diff:*)"
          "Bash(git show:*)"
          "Bash(git branch:*)"
          "Bash(git remote:*)"
          "Bash(git blame:*)"
          "Bash(git ls-files:*)"
          "Bash(git rev-parse:*)"
          "Bash(git describe:*)"
          "Bash(git shortlog:*)"
          "Bash(git reflog:*)"
          "Bash(git cat-file:*)"
          "Bash(git grep:*)"
          "Bash(git stash list:*)"
          "Bash(git worktree list:*)"
          "Bash(git config --get:*)"
          "Bash(git config --list:*)"
          "Bash(git config -l)"
          "Bash(git ls-tree:*)"
          "Bash(git show-ref:*)"
          "Bash(git for-each-ref:*)"
          "Bash(git rev-list:*)"
          "Bash(git merge-base:*)"
          "Bash(git name-rev:*)"
          "Bash(git submodule status:*)"

          # Safe file system operations
          "Bash(ls:*)"
          # NOTE: find/fd are read-only by default but can run mutating
          # commands via -exec/-delete (find) or -x/-X (fd). Trusted here for
          # workflow smoothness; tighten if exposed to untrusted prompts.
          "Bash(find:*)"
          "Bash(fd:*)"
          "Bash(cat:*)"
          "Bash(head:*)"
          "Bash(tail:*)"
          "Bash(pwd)"
          "Bash(stat:*)"
          "Bash(file:*)"
          "Bash(wc:*)"
          "Bash(tree:*)"
          "Bash(realpath:*)"
          "Bash(readlink:*)"
          "Bash(dirname:*)"
          "Bash(basename:*)"
          "Bash(du:*)"
          "Bash(df:*)"

          # Safe read-only text/data inspection
          "Bash(rg:*)"
          "Bash(grep:*)"
          "Bash(diff:*)"
          "Bash(sort:*)"
          "Bash(uniq:*)"
          "Bash(cut:*)"
          "Bash(comm:*)"
          "Bash(column:*)"
          "Bash(jq:*)"
          "Bash(nl:*)"
          "Bash(tac:*)"
          "Bash(rev:*)"
          "Bash(tr:*)"
          # NOTE: -n suppresses default output and blocks in-place edits, but
          # `w`/`s///w`/`W` commands can still write a file. Obscure; kept for
          # parity with the codex allowlist.
          "Bash(sed -n:*)"

          # Safe read-only binary/hash inspection
          "Bash(od:*)"
          "Bash(xxd:*)"
          "Bash(hexdump:*)"
          "Bash(strings:*)"
          "Bash(base64:*)"
          "Bash(cksum:*)"
          "Bash(md5sum:*)"
          "Bash(sha1sum:*)"
          "Bash(sha256sum:*)"
          "Bash(sha512sum:*)"
          "Bash(b2sum:*)"

          # Safe read-only system info
          "Bash(whoami)"
          "Bash(id)"
          "Bash(id:*)"
          "Bash(hostname)"
          "Bash(uname:*)"
          "Bash(date)"
          "Bash(date:*)"
          "Bash(uptime)"
          "Bash(env)"
          "Bash(printenv:*)"
          "Bash(which:*)"
          "Bash(type:*)"
          "Bash(command -v:*)"
          "Bash(getconf:*)"
          "Bash(free:*)"
          "Bash(ps:*)"
          "Bash(pgrep:*)"
          "Bash(lsof:*)"
          "Bash(ss:*)"
          "Bash(lscpu)"
          "Bash(lscpu:*)"
          "Bash(lsblk:*)"
          "Bash(lsusb:*)"
          "Bash(lspci:*)"
          "Bash(findmnt:*)"
          "Bash(getent:*)"
          "Bash(groups)"
          "Bash(groups:*)"
          "Bash(locale)"
          "Bash(locale:*)"

          # Safe nix read operations
          "Bash(nix eval:*)"
          "Bash(nix flake show:*)"
          "Bash(nix flake metadata:*)"
          "Bash(nix search:*)"
          "Bash(nix log:*)"
          "Bash(nix path-info:*)"
          "Bash(nix derivation show:*)"
          "Bash(nix why-depends:*)"
          "Bash(nix store ls:*)"
          "Bash(nix store cat:*)"
          "Bash(nix config show:*)"
          "Bash(nix show-config:*)"
          "Bash(nix registry list:*)"
          "Bash(nix profile list:*)"
          "Bash(nix store info:*)"
          "Bash(nix-instantiate --parse:*)"
          "Bash(nix-store -q:*)"
          "Bash(nix-store --query:*)"
          "Bash(nixos-option:*)"
          "Bash(statix check:*)"
          "Bash(nh search:*)"

          # Jujutsu read-only (jj-toolkit skill)
          "Bash(jj status:*)"
          "Bash(jj log:*)"
          "Bash(jj diff:*)"
          "Bash(jj show:*)"
          "Bash(jj evolog:*)"
          "Bash(jj op log:*)"
          "Bash(jj file list:*)"
          "Bash(jj bookmark list:*)"

          # GitHub CLI read-only (github-toolkit skill); gh api stays on ask
          # since it can mutate via -X POST/PATCH/DELETE.
          "Bash(gh pr view:*)"
          "Bash(gh pr list:*)"
          "Bash(gh pr diff:*)"
          "Bash(gh pr checks:*)"
          "Bash(gh pr status:*)"
          "Bash(gh issue view:*)"
          "Bash(gh issue list:*)"
          "Bash(gh issue status:*)"
          "Bash(gh run list:*)"
          "Bash(gh run view:*)"
          "Bash(gh repo view:*)"
          "Bash(gh release list:*)"
          "Bash(gh release view:*)"
          "Bash(gh label list:*)"
          "Bash(gh search:*)"

          # MCP tools - read only
          "mcp__github__search_repositories"
          "mcp__github__get_file_contents"
          "mcp__sequential-thinking__sequentialthinking"

          # Filesystem MCP - read operations
          "mcp__filesystem__read_file"
          "mcp__filesystem__read_text_file"
          "mcp__filesystem__read_media_file"
          "mcp__filesystem__read_multiple_files"
          "mcp__filesystem__list_directory"
          "mcp__filesystem__list_directory_with_sizes"
          "mcp__filesystem__directory_tree"
          "mcp__filesystem__search_files"
          "mcp__filesystem__get_file_info"
          "mcp__filesystem__list_allowed_directories"

          # Git MCP - read-only operations
          "mcp__git__git_status"
          "mcp__git__git_log"
          "mcp__git__git_diff"
          "mcp__git__git_diff_staged"
          "mcp__git__git_diff_unstaged"
          "mcp__git__git_show"
          "mcp__git__git_branch"

          # Fetch / Tavily MCP - read-only web
          "mcp__fetch__fetch"
          "mcp__tavily__tavily-search"
          "mcp__tavily__tavily-extract"
          "mcp__tavily__tavily-map"

          # Trusted web domains
          "WebFetch(domain:github.com)"
          "WebFetch(domain:raw.githubusercontent.com)"
          "WebFetch(domain:devenv.sh)"

          # Git staging
          "Bash(git add:*)"

          # Nix evaluation/check (can trigger builds)
          "Bash(nix flake check:*)"

          # Directory creation
          "Bash(mkdir:*)"
          "Bash(chmod:*)"

          # System info
          "Bash(systemctl list-units:*)"
          "Bash(systemctl list-timers:*)"
          "Bash(systemctl status:*)"
          "Bash(journalctl:*)"
          "Bash(dmesg:*)"
          "Bash(claude --version)"

          # Debugging
          "Bash(coredumpctl list:*)"
        ];
        ask = [
          # Potentially destructive git commands
          "Bash(git checkout:*)"
          "Bash(git commit:*)"
          "Bash(git merge:*)"
          "Bash(git pull:*)"
          "Bash(git push:*)"
          "Bash(git rebase:*)"
          "Bash(git reset:*)"
          "Bash(git restore:*)"
          "Bash(git stash:*)"
          "Bash(git switch:*)"

          # File deletion and modification
          "Bash(cp:*)"
          "Bash(mv:*)"
          "Bash(rm:*)"
          # Phase 1 destructive-command baseline is ask for explicit primitives.
          "Bash(rm -rf:*)"
          "Bash(dd:*)"
          "Bash(mkfs:*)"
          "Bash(shutdown)"
          "Bash(shutdown:*)"
          "Bash(reboot)"
          "Bash(reboot:*)"

          # System control operations
          "Bash(systemctl disable:*)"
          "Bash(systemctl enable:*)"
          "Bash(systemctl mask:*)"
          "Bash(systemctl reload:*)"
          "Bash(systemctl restart:*)"
          "Bash(systemctl start:*)"
          "Bash(systemctl stop:*)"
          "Bash(systemctl unmask:*)"

          # Network operations
          "Bash(curl:*)"
          "Bash(ping:*)"
          "Bash(rsync:*)"
          "Bash(scp:*)"
          "Bash(ssh:*)"
          "Bash(wget:*)"

          # Package management
          "Bash(nix build:*)"
          "Bash(nix run:*)"
          "Bash(nix shell:*)"
          "Bash(nixos-rebuild:*)"
          "Bash(sudo:*)"

          # Process management
          "Bash(kill:*)"
          "Bash(killall:*)"
          "Bash(pkill:*)"
        ];
        deny = [
          "Bash(rm -rf /*)"
          "Bash(rm -rf /)"
        ];
      };
    };
    lspServers = {
      bashls = {
        command = getExe pkgs.bash-language-server;
        args = [ "start" ];
        extensionToLanguage = toLang "shellscript" [
          ".sh"
          ".bash"
        ];
      };

      basedpyright = {
        command = getExe' pkgs.basedpyright "basedpyright-langserver";
        args = [ "--stdio" ];
        extensionToLanguage = toLang "python" [
          ".py"
          ".pyi"
          ".pyw"
        ];
      };

      clangd = {
        command = getExe' pkgs.clang-tools "clangd";
        args = [
          "--background-index"
          "--clang-tidy"
          "--header-insertion=iwyu"
          "--completion-style=detailed"
          "--function-arg-placeholders"
          "--fallback-style=llvm"
        ];
        initializationOptions = {
          usePlaceholders = true;
          completeUnimported = true;
          clangdFileStatus = true;
        };
        extensionToLanguage =
          (toLang "c" [
            ".c"
            ".h"
          ])
          // (toLang "cpp" [
            ".cpp"
            ".cc"
            ".cxx"
            ".c++"
            ".hpp"
            ".hh"
            ".hxx"
            ".h++"
          ]);
      };

      cmake = {
        command = getExe pkgs.cmake-language-server;
        extensionToLanguage = toLang "cmake" [ ".cmake" ];
      };

      cssls = {
        command = getExe' pkgs.vscode-langservers-extracted "vscode-css-language-server";
        args = [ "--stdio" ];
        extensionToLanguage = {
          ".css" = "css";
          ".scss" = "scss";
          ".less" = "less";
        };
      };

      emmylua-ls = {
        command = getExe pkgs.emmylua-ls;
        extensionToLanguage = toLang "lua" [ ".lua" ];
      };

      fish-lsp = {
        command = getExe pkgs.fish-lsp;
        extensionToLanguage = toLang "fish" [ ".fish" ];
      };

      html = {
        command = getExe' pkgs.vscode-langservers-extracted "vscode-html-language-server";
        args = [ "--stdio" ];
        extensionToLanguage = toLang "html" [
          ".html"
          ".htm"
        ];
      };

      jsonls = {
        command = getExe' pkgs.vscode-langservers-extracted "vscode-json-language-server";
        args = [ "--stdio" ];
        extensionToLanguage = {
          ".json" = "json";
          ".jsonc" = "jsonc";
        };
      };

      julia = {
        command = lib.getExe pkgs.julia-bin;
        args = [
          "--startup-file=no"
          "--history-file=no"
          "--quiet"
          "--project=@languageserver"
          "-e"
          "using LanguageServer; runserver()"
        ];
        extensionToLanguage = toLang "julia" [ ".jl" ];
        startupTimeout = 90000;
      };

      latex = {
        command = lib.getExe pkgs.texlab;
        extensionToLanguage = {
          ".bib" = "bibtex";
          ".cls" = "latex";
          ".sty" = "latex";
          ".tex" = "latex";
        };
        transport = "stdio";
      };

      marksman = {
        command = getExe pkgs.marksman;
        extensionToLanguage = toLang "markdown" [
          ".md"
          ".markdown"
          ".mdx"
        ];
      };

      nixd = {
        command = getExe pkgs.nixd;
        extensionToLanguage = toLang "nix" [ ".nix" ];
      };

      ruff = {
        command = getExe pkgs.ruff;
        args = [ "server" ];
        extensionToLanguage = toLang "python" [
          ".py"
          ".pyi"
          ".pyw"
        ];
      };

      rust-analyzer = {
        command = getExe pkgs.rust-analyzer;
        extensionToLanguage = toLang "rust" [ ".rs" ];
      };

      taplo = {
        command = getExe pkgs.taplo;
        args = [
          "lsp"
          "stdio"
        ];
        extensionToLanguage = toLang "toml" [ ".toml" ];
      };

      typst = {
        command = lib.getExe pkgs.tinymist;
        extensionToLanguage = toLang "typst" [ ".typ" ];
      };

      yamlls = {
        command = getExe pkgs.yaml-language-server;
        args = [ "--stdio" ];
        extensionToLanguage = toLang "yaml" [
          ".yaml"
          ".yml"
        ];
      };
    };
  };

  preservation.preserveAt.state-dir.directories = [ ".claude" ];
}
