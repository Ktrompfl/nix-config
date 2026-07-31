{
  programs.claude-code.skills = {
    ast-grep = ''
      ---
      name: ast-grep
      description: Use ast-grep for structural, language-aware code search instead of grep/manual file reads whenever navigating or refactoring code across a codebase. Triggers on "find all callers of X", "find every place that does Y pattern", "rename/refactor across files", "structural search", or before reaching for grep on anything more than a literal string. Falls back to ripgrep for pure text/comment/string searches ast-grep can't express.
      ---

      # ast-grep navigation

      `ast-grep` parses code into an AST and matches structural patterns, so it finds real occurrences of a construct instead of text that happens to look like it (skips comments/strings, respects language syntax). It is already Bash-allowlisted (`Bash(ast-grep *)`) in this config.

      ## When to reach for it
      - Locating a function/method definition or every call site across many files
      - Refactoring searches: "every place a struct/component is constructed with these fields"
      - Any pattern with "shape" (call with N args, import of X, JSX element with a prop) rather than a literal string

      Use plain `rg`/`grep` instead for literal strings, comments, config files, or one-off greps — don't reach for ast-grep just because it's available.

      ## Core commands

      Search without writing a config file:
      ```
      ast-grep run -p '<pattern>' [-l <lang>] [<path>]
      ```

      Metavariables in patterns:
      - `$NAME` matches a single node (identifier, expression, etc.)
      - `$$$ARGS` matches zero or more nodes (argument lists, statement lists)
      - Reuse the same `$NAME` twice in a pattern to require the same node in both places

      Examples:
      ```
      ast-grep run -p 'console.log($$$ARGS)' -l js
      ast-grep run -p 'def $NAME($$$ARGS):' -l python
      ast-grep run -p 'useState($$$ARGS)' -l tsx
      ```

      Add `--json` when you need to post-process matches (counts, dedupe, feed into another step) instead of reading raw text — keeps result parsing cheap.

      Rewriting across a codebase:
      ```
      ast-grep run -p '<pattern>' -r '<rewrite>' [-l <lang>] --update-all
      ```
      Only pass `--update-all` after confirming the plain search results look right — treat it like any other bulk edit that needs review.

      ## Efficiency notes
      - One `ast-grep run` over the whole repo replaces many rounds of `grep` + manual `Read` of candidate files — prefer it when a task would otherwise mean opening several files just to check "does this match the pattern."
      - Scope with `<path>` or `--globs` when the repo is large, rather than grepping the whole tree and filtering by hand afterward.
      - `-l <lang>` is optional (ast-grep infers it from the file extension) but pin it when scanning mixed-language directories to avoid cross-language false matches.
    '';

    conventional-commits = ''
      ---
      name: conventional-commits
      description: Use when writing a git commit message in this repo. Produces a compact Conventional Commits header (and only a body/footer when one is actually needed) without the token overhead of a full commit-message essay. Triggers whenever the user asks to commit, write a commit message, or follow conventional commits.
      ---

      # Conventional commits, kept cheap

      Format the header as:
      ```
      <type>(<scope>)!: <description>
      ```
      - `<scope>` and `!` (breaking-change marker) are optional — omit both when they don't add information
      - `<description>`: imperative mood, lowercase start, no trailing period, aim for <=72 chars total header
      - Pick `<type>` from: `feat`, `fix`, `refactor`, `perf`, `test`, `build`, `ci`, `docs`, `style`, `chore`

      ## Body/footer — only when they earn their keep
      - Skip the body entirely for small, self-explanatory changes; the header is the whole commit message
      - Add a short body (1-2 lines, why not what) only if the change isn't obvious from the diff/header
      - Add a footer only for `BREAKING CHANGE: ...` or `Fixes #123`-style references — don't restate the diff

      Do not pad the message with a bullet-point summary of every file changed — that's what `git show` is for. The goal is the smallest message that correctly categorizes and explains the change, not a changelog entry.
    '';
  };
}
