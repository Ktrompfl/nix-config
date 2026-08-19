vim.g.mapleader = " "
vim.g.maplocalleader = " "
vim.g.have_nerd_font = true
vim.loader.enable()

-- Built-in gra/gri/grn/grr/grt LSP maps would otherwise fight mini.operators' `gr`
-- operator (plugins/mini-edit.nix); <Leader>l* (keymaps.nix) already covers all of them.
vim.g.lsp_no_default_maps = true

vim.o.mouse = "a"
vim.o.switchbuf = "usetab"
vim.o.undofile = true
vim.o.shada = "'100,<50,s10,:1000,/100,@100,h" -- limit ShaDa file for faster startup

vim.cmd("filetype plugin indent on")
if vim.fn.exists("syntax_on") ~= 1 then
	vim.cmd("syntax enable")
end

vim.o.breakindent = true
vim.o.breakindentopt = "list:-1"
vim.o.colorcolumn = "+1"
vim.o.cursorline = true
vim.o.linebreak = true
vim.o.list = true
vim.o.number = true
vim.o.relativenumber = true
vim.o.pumheight = 10
vim.o.ruler = false
vim.o.shortmess = "CFOSWaco"
vim.o.showmode = false
vim.o.signcolumn = "yes"
vim.o.splitbelow = true
vim.o.splitkeep = "screen"
vim.o.splitright = true
vim.o.winborder = "single"
vim.o.wrap = false
vim.o.cursorlineopt = "screenline,number"
vim.o.fillchars = "eob: ,fold:╌"
vim.opt.listchars = { tab = "» ", trail = "·", nbsp = "␣", extends = "…", precedes = "…" }

-- Nothing folded by default; treesitter.nix sets foldmethod/foldexpr per-buffer
vim.o.foldlevel = 10
vim.o.foldmethod = "indent"
vim.o.foldnestmax = 10
vim.o.foldtext = ""

vim.o.autoindent = true
vim.o.expandtab = true
vim.o.formatoptions = "rqnl1j"
vim.o.ignorecase = true
vim.o.incsearch = true
vim.o.infercase = true
vim.o.shiftwidth = 2
vim.o.smartcase = true
vim.o.smartindent = true
vim.o.spelloptions = "camel"
vim.o.tabstop = 2
vim.o.virtualedit = "block"
vim.o.iskeyword = "@,48-57,_,192-255,-" -- dash is part of the `word` textobject
vim.o.formatlistpat = [[^\s*[0-9\-\+\*]\+[\.\)]*\s\+]] -- list-item start, used by `gw`

vim.o.complete = ".,w,b,kspell"
vim.o.completeopt = "menuone,noselect,fuzzy,nosort"

vim.schedule(function()
	vim.o.clipboard = "unnamedplus"
end) -- deferred: avoids slowing startup
vim.o.inccommand = "split"
vim.o.scrolloff = 10
vim.o.confirm = true -- prompt to save instead of failing e.g. `:q` with unsaved changes
