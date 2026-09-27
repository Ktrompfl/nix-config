{
  apps.claude.files = {
    ".claude/skills/ast-grep/SKILL.md".text = ''
      ---
      name: ast-grep
      description: Structural, syntax-aware code search and rewrite. Use for finding definitions, call sites or code of a given shape, and for cross-file refactors; use rg for literal text, comments and config.
      ---

      - Search: `ast-grep run -p '<pattern>' [-l <lang>] [<path>]`
      - Rewrite: add `-r '<rewrite>' --update-all`, only after checking the plain search.
      - `$X` matches one node, `$$$X` zero or more; a repeated `$X` must match the same node.
        e.g. `def $F($$$A):` (python), `console.log($$$A)` (js)
      - `--json` for post-processing; `<path>`/`--globs` to scope; `-l` in mixed-language trees.
      - One run replaces rounds of grep plus reading candidate files.
    '';

    ".claude/skills/conventional-commits/SKILL.md".text = ''
      ---
      name: conventional-commits
      description: Compact Conventional Commits message. Use whenever writing a commit message.
      ---

      - Header: `<type>(<scope>)!: <description>`; scope and `!` only when informative.
      - Types: feat, fix, refactor, perf, test, build, ci, docs, style, chore.
      - Description: imperative, lowercase, no period, header ≤ 72 chars.
      - Body only if the why isn't obvious (1-2 lines); footer only for `BREAKING CHANGE:` or `Fixes #N`.
      - Never list changed files.
    '';
  };
}
