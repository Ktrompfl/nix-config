-- ==== mini-core ===================================================

-- Plugin manager; Nix installs the plugins, this only provides staged `now`/`later`
-- loading (plus graceful error handling) for startup performance.
require("mini.deps").setup()
local now, later = MiniDeps.now, MiniDeps.later
-- Some setup only needs `now()` when started like `nvim path/to/file`
local now_if_args = vim.fn.argc(-1) > 0 and now or later

now(function()
	require("mini.basics").setup({
		options = { basic = false }, -- options are set in options.nix instead
		mappings = { windows = true, move_with_alt = true },
	})
end)

now(function()
	require("mini.icons").setup({ style = "glyph" })
end)

now_if_args(function()
	require("mini.misc").setup()
	MiniMisc.setup_auto_root() -- cd to nearest .git/Makefile ancestor
	MiniMisc.setup_restore_cursor()
	MiniMisc.setup_termbg_sync()
end)

now(function()
	require("mini.notify").setup()
end)

now(function()
	require("mini.sessions").setup()
end)

-- Extra pickers/textobjects/highlighters used by other mini.* modules; must load
-- before those (mini-edit.nix's mini.ai, mini-ui.nix's mini.hipatterns), so it lives
-- here in mini-core.nix, which plugins/default.nix imports first.
later(function()
	require("mini.extra").setup()
end)

-- ==== mini-ui =====================================================

now(function()
	require("mini.statusline").setup()
end)

now(function()
	require("mini.tabline").setup()
end)

later(function()
	require("mini.cursorword").setup()
end)

later(function()
	require("mini.indentscope").setup()
end)

later(function()
	local hipatterns = require("mini.hipatterns")
	local hi_words = MiniExtra.gen_highlighter.words
	hipatterns.setup({
		highlighters = {
			fixme = hi_words({ "FIXME", "Fixme", "fixme" }, "MiniHipatternsFixme"),
			hack = hi_words({ "HACK", "Hack", "hack" }, "MiniHipatternsHack"),
			todo = hi_words({ "TODO", "Todo", "todo" }, "MiniHipatternsTodo"),
			note = hi_words({ "NOTE", "Note", "note" }, "MiniHipatternsNote"),
			hex_color = hipatterns.gen_highlighter.hex_color(),
		},
	})
end)

-- Shows next-key hints on `<Leader>`, `\`, `[`/`]`, `` ` ``/`'`, `"`, `<C-w>`, `g`, `z`
later(function()
	local miniclue = require("mini.clue")
  -- stylua: ignore
  miniclue.setup({
    clues = {
      leader_group_clues, -- from keymaps.nix
      miniclue.gen_clues.builtin_completion(),
      miniclue.gen_clues.g(),
      miniclue.gen_clues.marks(),
      miniclue.gen_clues.registers(),
      miniclue.gen_clues.windows({ submode_resize = true }),
      miniclue.gen_clues.z(),
    },
    triggers = {
      { mode = 'n', keys = '<Leader>' }, { mode = 'x', keys = '<Leader>' },
      { mode = 'n', keys = '\\' },
      { mode = 'n', keys = '[' }, { mode = 'n', keys = ']' },
      { mode = 'x', keys = '[' }, { mode = 'x', keys = ']' },
      { mode = 'i', keys = '<C-x>' },
      { mode = 'n', keys = 'g' }, { mode = 'x', keys = 'g' },
      { mode = 'n', keys = "'" }, { mode = 'n', keys = '`' },
      { mode = 'x', keys = "'" }, { mode = 'x', keys = '`' },
      { mode = 'n', keys = '"' }, { mode = 'x', keys = '"' },
      { mode = 'i', keys = '<C-r>' }, { mode = 'c', keys = '<C-r>' },
      { mode = 'n', keys = '<C-w>' },
      { mode = 'n', keys = 'z' }, { mode = 'x', keys = 'z' },
    },
  })
end)

-- ==== mini-edit ===================================================

-- a/i textobjects, e.g. `ci)`, `yaq`, `vif`, `cina` (next argument)
later(function()
	local ai = require("mini.ai")
	ai.setup({
		custom_textobjects = {
			B = MiniExtra.gen_ai_spec.buffer(),
			F = ai.gen_spec.treesitter({ a = "@function.outer", i = "@function.inner" }),
		},
		search_method = "cover", -- only match the textobject covering the cursor
	})
end)

-- `ga`/`gA` (interactive) align operators
later(function()
	require("mini.align").setup()
end)

-- `[`/`]` + target (b buffer, c conflict, d diagnostic, ...) navigation
later(function()
	require("mini.bracketed").setup()
end)

later(function()
	require("mini.bufremove").setup()
end)

-- `gc` comment operator, e.g. `gcip`
later(function()
	require("mini.comment").setup()
end)

later(function()
	-- Don't show noisy 'Text' suggestions; show snippets last
	local process_items_opts = { kind_priority = { Text = -1, Snippet = 99 } }
	local process_items = function(items, base)
		return MiniCompletion.default_process_items(items, base, process_items_opts)
	end
	require("mini.completion").setup({
		lsp_completion = { source_func = "omnifunc", auto_setup = false, process_items = process_items },
	})
	new_autocmd("LspAttach", nil, function(ev)
		vim.bo[ev.buf].omnifunc = "v:lua.MiniCompletion.completefunc_lsp"
	end, "Set 'omnifunc'")
	vim.lsp.config("*", { capabilities = MiniCompletion.get_lsp_capabilities() })
end)

-- `<CR>` labels every visible jump spot; type the label to land on it. Also works as
-- an operator target, e.g. `d<CR>` deletes up to the chosen spot.
later(function()
	require("mini.jump2d").setup()
end)

-- Navigate 'mini.completion' menu with `<Tab>`/`<S-Tab>`; account for 'mini.pairs' on `<CR>`/`<BS>`
later(function()
	require("mini.keymap").setup()
	MiniKeymap.map_multistep("i", "<Tab>", { "pmenu_next" })
	MiniKeymap.map_multistep("i", "<S-Tab>", { "pmenu_prev" })
	MiniKeymap.map_multistep("i", "<CR>", { "pmenu_accept", "minipairs_cr" })
	MiniKeymap.map_multistep("i", "<BS>", { "minipairs_bs" })
end)

-- `<M-hjkl>` moves the current line/selection
later(function()
	require("mini.move").setup()
end)

-- `gr` replace, `gx` exchange, `gm` multiply, `gs` sort, `g=` evaluate-as-Lua, e.g.
-- `griw` replaces inside word. Auto-remaps built-in `gx` (open URL) to `gX`.
later(function()
	require("mini.operators").setup()
end)

later(function()
	require("mini.pairs").setup({ modes = { command = true } })
end)

-- `gS` toggles single-line/multi-line arguments
later(function()
	require("mini.splitjoin").setup()
end)

-- `sa`/`sd`/`sr`/`sf`/`sh` add/delete/replace/find/highlight surroundings, e.g. `saiw)`
later(function()
	require("mini.surround").setup()
end)

later(function()
	require("mini.trailspace").setup()
end)

-- ==== mini-workflow ===============================================

-- In-buffer diff hunks vs Git index; `gh`/`gH` apply/reset, `<Leader>go` toggles overlay
later(function()
	require("mini.diff").setup()
end)

-- Miller-columns file browser, edited as text; `<Leader>ed`/`<Leader>ef` open it
later(function()
	require("mini.files").setup({ windows = { preview = true } })
	new_autocmd("User", "MiniFilesExplorerOpen", function()
		MiniFiles.set_bookmark("c", vim.fn.stdpath("config"), { desc = "Config" })
		MiniFiles.set_bookmark("w", vim.fn.getcwd, { desc = "Working directory" })
	end, "Add bookmarks")
end)

-- `:Git` command + `<Leader>gs` show-at-cursor; not a full git client, mini.diff covers hunks
later(function()
	require("mini.git").setup()
end)

-- Fuzzy picker (fzf-equivalent) backed by ripgrep; see the `<Leader>f*` mappings
later(function()
	require("mini.pick").setup()
end)

-- Frecency-tracked file visits, `<Leader>fv`/`<Leader>v*`
later(function()
	require("mini.visits").setup()
end)

-- ==== treesitter ==================================================

-- ==== lsp =========================================================

vim.lsp.enable("clangd")

-- Julia: JETLS (new) or classic LanguageServer.jl - both read from
-- '~/.julia/environments/languageserver' (bootstrapped in julia.nix, also used
-- by Claude Code's own LSP wiring in claude.nix). Flip to switch.
local julia_use_jetls = true
if julia_use_jetls then
	vim.lsp.config("jetls", {
		cmd = { vim.fn.expand("~/.julia/bin/jetls"), "serve" },
		filetypes = { "julia" },
		root_markers = { "Project.toml" },
	})
	vim.lsp.enable("jetls")
else
	vim.lsp.config("julials", {
		cmd = {
			"julia",
			"--startup-file=no",
			"--history-file=no",
			"--quiet",
			"--project=" .. vim.fn.expand("~/.julia/environments/languageserver"),
			"-e",
			"using LanguageServer; runserver()",
		},
	})
	vim.lsp.enable("julials")
end

vim.lsp.enable("lua_ls")
vim.lsp.enable("stylua")

vim.lsp.enable("markdown_oxide")
vim.lsp.enable("marksman")

vim.lsp.config("nixd", {
	settings = {
		nixd = {
			nixpkgs = { expr = "import <nixpkgs> { }" },
			formatting = { command = { "nixfmt" } },
		},
	},
})
vim.lsp.enable("nixd")

vim.lsp.enable("ty")
vim.lsp.enable("ruff")

vim.lsp.enable("rust_analyzer")
vim.lsp.enable("texlab")

vim.lsp.config("tinymist", {
	settings = { exportPdf = "onSave", formatterMode = "typstyle" },
})
vim.lsp.enable("tinymist")

vim.lsp.enable("yamlls")

-- Spelling/grammar for prose and code comments
vim.lsp.config("harper_ls", { settings = { ["harper-ls"] = { dialect = "British" } } })
vim.lsp.enable("harper_ls")

-- Code lens (inline "N references"/"Run test" annotations, e.g. rust-analyzer, JETLS).
-- Not on by default; unlike inlay hints there's no vim.o toggle, only per-buffer enable.
new_autocmd("LspAttach", nil, function(args)
	local client = vim.lsp.get_client_by_id(args.data.client_id)
	if client and client:supports_method("textDocument/codeLens") then
		vim.lsp.codelens.enable(true, { bufnr = args.buf })
	end
end, "Code lens enable")

-- ==== formatting ==================================================

later(function()
	require("conform").setup({
		format_on_save = { lsp_fallback = true, timeout_ms = 2000 },
		formatters_by_ft = {
			julia = { "runic" },
			lua = { "stylua" },
			nix = { "nixfmt" },
			plaintex = { "latexindent" },
			python = { "ruff_organize_imports", "ruff_format" },
			rust = { "rustfmt" },
			tex = { "latexindent" },
		},
	})
end)

-- ==== docs ========================================================

-- Edits '.ipynb' as plain '# %%'-delimited text via the 'jupytext' CLI.
-- Uses `now()` (not `later()`) so it also works when started like `nvim nb.ipynb`.
now_if_args(function()
	require("jupytext").setup({ style = "hydrogen" })
end)

later(function()
	require("render-markdown").setup({})
end)

later(function()
	require("typst-preview").setup({
		port = 0,
		follow_cursor = true,
		dependencies_bin = { ["tinymist"] = "tinymist", ["websocat"] = "websocat" },
	})
end)

-- ==== claude ======================================================

-- Starts a local server any `claude` CLI - including one in another terminal - auto-discovers,
-- so selections/diffs/`@file` mentions work either way. 'native' avoids needing snacks.nvim.
later(function()
	require("claudecode").setup({ terminal = { provider = "native" } })
end)
