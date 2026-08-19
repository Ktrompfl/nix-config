{ config, ... }:
[
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
  "Read(${config.directory}/.claude/**)"
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
  "Bash(ast-grep *)"
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

  "mcp__context7__get-library-docs"
  "mcp__context7__resolve-library-id"
  "mcp__nixos__*"

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
]
