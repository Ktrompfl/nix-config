_: [
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
]
