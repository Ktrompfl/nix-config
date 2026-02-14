{ pkgs, ... }:
{
  # does not work reliably for components like mini terminal
  # stylix.targets.neovim.transparentBackground = {
  #   main = true;
  #   signColumn = true;
  #   numberLine = true;
  # };

  programs.neovim = {
    enable = true;
    viAlias = true; # alias vi to nvim
    vimAlias = true; # alias vim to nvim
    vimdiffAlias = true; # alias vimdiff to nvim -d

    defaultEditor = true; # set nvim as default editor with the $EDITOR session variable

    extraPackages = with pkgs; [
      # core dependencies
      git
      ripgrep

      # language servers
      clang-tools
      lua-language-server # lua language server
      ltex-ls-plus
      markdown-oxide
      marksman
      # nil # nix language server
      nginx-language-server
      nixd
      basedpyright
      ruff
      texlab
      yaml-language-server

      # dependencies for typst-preview
      tinymist
      websocat

      # formatters
      stylua
      nixfmt
      nodePackages.prettier # general use formatter
      shfmt # shell parser and formatter
    ];

    initLua = /* lua */ ''
      -- Learn more about Neovim lua api
      -- https://neovim.io/doc/user/lua-guide.html

      -- [[ Setting options ]]

      -- Set <space> as the leader key
      -- See `:help mapleader`
      --  NOTE: Must happen before plugins are loaded (otherwise wrong leader will be used)
      vim.g.mapleader = ' '
      vim.g.maplocalleader = ' '

      -- Set to true if you have a Nerd Font installed and selected in the terminal
      vim.g.have_nerd_font = true

      -- Enable experimental Lua module loader to improve startup times
      vim.loader.enable()

      vim.o.mouse       = 'a'            -- Enable mouse
      -- vim.o.mousescroll = 'ver:25,hor:6' -- Customize mouse scroll
      vim.o.switchbuf   = 'usetab'       -- Use already opened buffers when switching
      vim.o.undofile    = true           -- Enable persistent undo

      vim.o.shada = "'100,<50,s10,:1000,/100,@100,h" -- Limit ShaDa file (for startup)

      -- Enable all filetype plugins and syntax (if not enabled, for better startup)
      vim.cmd('filetype plugin indent on')
      if vim.fn.exists('syntax_on') ~= 1 then vim.cmd('syntax enable') end

      -- UI
      vim.o.breakindent    = true       -- Indent wrapped lines to match line start
      vim.o.breakindentopt = 'list:-1'  -- Add padding for lists (if 'wrap' is set)
      vim.o.colorcolumn    = '+1'       -- Draw column on the right of maximum width
      vim.o.cursorline     = true       -- Enable current line highlighting
      vim.o.linebreak      = true       -- Wrap lines at 'breakat' (if 'wrap' is set)
      vim.o.list           = true       -- Show helpful text indicators
      vim.o.number         = true       -- Show line numbers
      vim.o.relativenumber = true       --
      vim.o.pumheight      = 10         -- Make popup menu smaller
      vim.o.ruler          = false      -- Don't show cursor coordinates
      vim.o.shortmess      = 'CFOSWaco' -- Disable some built-in completion messages
      vim.o.showmode       = false      -- Don't show mode in command line
      vim.o.signcolumn     = 'yes'      -- Always show signcolumn (less flicker)
      vim.o.splitbelow     = true       -- Horizontal splits will be below
      vim.o.splitkeep      = 'screen'   -- Reduce scroll during window split
      vim.o.splitright     = true       -- Vertical splits will be to the right
      vim.o.winborder      = 'single'   -- Use border in floating windows
      vim.o.wrap           = false      -- Don't visually wrap lines (toggle with \w)

      vim.o.cursorlineopt  = 'screenline,number' -- Show cursor line per screen line

      -- Special UI symbols. More is set via 'mini.basics' later.
      vim.o.fillchars = 'eob: ,fold:╌'
      vim.o.listchars = 'extends:…,nbsp:␣,precedes:…,tab:> '

      -- Folds (see `:h fold-commands`, `:h zM`, `:h zR`, `:h zA`, `:h zj`)
      vim.o.foldlevel   = 10       -- Fold nothing by default; set to 0 or 1 to fold
      vim.o.foldmethod  = 'indent' -- Fold based on indent level
      vim.o.foldnestmax = 10       -- Limit number of fold levels
      vim.o.foldtext    = ""       -- Show text under fold with its highlighting

      -- Editing
      vim.o.autoindent    = true    -- Use auto indent
      vim.o.expandtab     = true    -- Convert tabs to spaces
      vim.o.formatoptions = 'rqnl1j'-- Improve comment editing
      vim.o.ignorecase    = true    -- Ignore case during search
      vim.o.incsearch     = true    -- Show search matches while typing
      vim.o.infercase     = true    -- Infer case in built-in completion
      vim.o.shiftwidth    = 2       -- Use this number of spaces for indentation
      vim.o.smartcase     = true    -- Respect case if search pattern has upper case
      vim.o.smartindent   = true    -- Make indenting smart
      vim.o.spelloptions  = 'camel' -- Treat camelCase word parts as separate words
      vim.o.tabstop       = 2       -- Show tab as this number of spaces
      vim.o.virtualedit   = 'block' -- Allow going past end of line in blockwise mode

      vim.o.iskeyword = '@,48-57,_,192-255,-' -- Treat dash as `word` textobject part

      -- Pattern for a start of numbered list (used in `gw`). This reads as
      -- "Start of list item is: at least one special character (digit, -, +, *)
      -- possibly followed by punctuation (. or `)`) followed by at least one space".
      vim.o.formatlistpat = [[^\s*[0-9\-\+\*]\+[\.\)]*\s\+]]

      -- Built-in completion
      vim.o.complete    = '.,w,b,kspell'                  -- Use less sources
      vim.o.completeopt = 'menuone,noselect,fuzzy,nosort' -- Use custom behavior

      -- Sync clipboard between OS and Neovim.
      --  Schedule the setting after `UiEnter` because it can increase startup-time.
      --  Remove this option if you want your OS clipboard to remain independent.
      --  See `:help 'clipboard'`
      vim.schedule(function()
        vim.o.clipboard = 'unnamedplus'
      end)

      -- Sets how neovim will display certain whitespace characters in the editor.
      --  See `:help 'list'`
      --  and `:help 'listchars'`
      --
      --  Notice listchars is set using `vim.opt` instead of `vim.o`.
      --  It is very similar to `vim.o` but offers an interface for conveniently interacting with tables.
      --   See `:help lua-options`
      --   and `:help lua-options-guide`
      vim.o.list = true
      vim.opt.listchars = { tab = '» ', trail = '·', nbsp = '␣' }

      -- Preview substitutions live, as you type!
      vim.o.inccommand = 'split'

      -- Minimal number of screen lines to keep above and below the cursor.
      vim.o.scrolloff = 10

      -- if performing an operation that would fail due to unsaved changes in the buffer (like `:q`),
      -- instead raise a dialog asking if you wish to save the current file(s)
      -- See `:help 'confirm'`
      vim.o.confirm = true

      -- vim.o.autowrite = true

      -- [[ Keymaps ]]
      -- General mappings ===========================================================

      -- Use this section to add custom general mappings. See `:h vim.keymap.set()`.

      -- TIP: Disable arrow keys in normal mode
      -- vim.keymap.set('n', '<left>', '<cmd>echo "Use h to move!!"<CR>')
      -- vim.keymap.set('n', '<right>', '<cmd>echo "Use l to move!!"<CR>')
      -- vim.keymap.set('n', '<up>', '<cmd>echo "Use k to move!!"<CR>')
      -- vim.keymap.set('n', '<down>', '<cmd>echo "Use j to move!!"<CR>')

      -- An example helper to create a Normal mode mapping
      local nmap = function(lhs, rhs, desc)
        -- See `:h vim.keymap.set()`
        vim.keymap.set('n', lhs, rhs, { desc = desc })
      end

      -- Paste linewise before/after current line
      -- Usage: `yiw` to yank a word and `]p` to put it on the next line.
      nmap('[p', '<Cmd>exe "put! " . v:register<CR>', 'Paste Above')
      nmap(']p', '<Cmd>exe "put "  . v:register<CR>', 'Paste Below')

      -- Many general mappings are created by 'mini.basics'. See 'plugin/30_mini.lua'

      -- stylua: ignore start
      -- The next part (until `-- stylua: ignore end`) is aligned manually for easier
      -- reading. Consider preserving this or remove `-- stylua` lines to autoformat.

      -- Leader mappings

      -- Neovim has the concept of a Leader key (see `:h <Leader>`). It is a configurable
      -- key that is primarily used for "workflow" mappings (opposed to text editing).
      -- Like "open file explorer", "create scratch buffer", "pick from buffers".
      --
      -- <Leader> is set to <Space>, i.e. press <Space>
      -- whenever there is a suggestion to press <Leader>.
      --
      -- This config uses a "two key Leader mappings" approach: first key describes
      -- semantic group, second key executes an action. Both keys are usually chosen
      -- to create some kind of mnemonic.
      -- Example: `<Leader>f` groups "find" type of actions; `<Leader>ff` - find files.
      -- Use this section to add Leader mappings in a structural manner.
      --
      -- Usually if there are global and local kinds of actions, lowercase second key
      -- denotes global and uppercase - local.
      -- Example: `<Leader>fs` / `<Leader>fS` - find workspace/document LSP symbols.
      --
      -- Many of the mappings use 'mini.nvim' modules set up in below.

      -- Create a global table with information about Leader groups in certain modes.
      -- This is used to provide 'mini.clue' with extra clues.
      -- Add an entry if you create a new group.
      leader_group_clues = {
        { mode = 'n', keys = '<Leader>b', desc = '+Buffer' },
        { mode = 'n', keys = '<Leader>e', desc = '+Explore/Edit' },
        { mode = 'n', keys = '<Leader>f', desc = '+Find' },
        { mode = 'n', keys = '<Leader>g', desc = '+Git' },
        { mode = 'n', keys = '<Leader>l', desc = '+Language' },
        { mode = 'n', keys = '<Leader>m', desc = '+Map' },
        { mode = 'n', keys = '<Leader>o', desc = '+Other' },
        { mode = 'n', keys = '<Leader>s', desc = '+Session' },
        { mode = 'n', keys = '<Leader>t', desc = '+Terminal' },
        { mode = 'n', keys = '<Leader>v', desc = '+Visits' },

        { mode = 'x', keys = '<Leader>g', desc = '+Git' },
        { mode = 'x', keys = '<Leader>l', desc = '+Language' },
      }

      -- Helpers for a more concise `<Leader>` mappings.
      -- Most of the mappings use `<Cmd>...<CR>` string as a right hand side (RHS) in
      -- an attempt to be more concise yet descriptive. See `:h <Cmd>`.
      -- This approach also doesn't require the underlying commands/functions to exist
      -- during mapping creation: a "lazy loading" approach to improve startup time.
      local nmap_leader = function(suffix, rhs, desc)
        vim.keymap.set('n', '<Leader>' .. suffix, rhs, { desc = desc })
      end
      local xmap_leader = function(suffix, rhs, desc)
        vim.keymap.set('x', '<Leader>' .. suffix, rhs, { desc = desc })
      end

      -- b is for 'Buffer'. Common usage:
      -- - `<Leader>bs` - create scratch (temporary) buffer
      -- - `<Leader>ba` - navigate to the alternative buffer
      -- - `<Leader>bw` - wipeout (fully delete) current buffer
      local new_scratch_buffer = function()
        vim.api.nvim_win_set_buf(0, vim.api.nvim_create_buf(true, true))
      end

      nmap_leader('ba', '<Cmd>b#<CR>',                                 'Alternate')
      nmap_leader('bd', '<Cmd>lua MiniBufremove.delete()<CR>',         'Delete')
      nmap_leader('bD', '<Cmd>lua MiniBufremove.delete(0, true)<CR>',  'Delete!')
      nmap_leader('bs', new_scratch_buffer,                            'Scratch')
      nmap_leader('bw', '<Cmd>lua MiniBufremove.wipeout()<CR>',        'Wipeout')
      nmap_leader('bW', '<Cmd>lua MiniBufremove.wipeout(0, true)<CR>', 'Wipeout!')

      -- e is for 'Explore' and 'Edit'. Common usage:
      -- - `<Leader>ed` - open explorer at current working directory
      -- - `<Leader>ef` - open directory of current file (needs to be present on disk)
      -- - `<Leader>ei` - edit 'init.lua'
      -- - All mappings that use `edit_plugin_file` - edit 'plugin/' config files
      local explore_at_file = '<Cmd>lua MiniFiles.open(vim.api.nvim_buf_get_name(0))<CR>'
      local explore_quickfix = function()
        for _, win_id in ipairs(vim.api.nvim_tabpage_list_wins(0)) do
          if vim.fn.getwininfo(win_id)[1].quickfix == 1 then return vim.cmd('cclose') end
        end
        vim.cmd('copen')
      end

      nmap_leader('ed', '<Cmd>lua MiniFiles.open()<CR>',          'Directory')
      nmap_leader('ef', explore_at_file,                          'File directory')
      nmap_leader('ei', '<Cmd>edit $MYVIMRC<CR>',                 'init.lua')
      nmap_leader('en', '<Cmd>lua MiniNotify.show_history()<CR>', 'Notifications')
      nmap_leader('eq', explore_quickfix,                         'Quickfix')

      -- f is for 'Fuzzy Find'. Common usage:
      -- - `<Leader>ff` - find files; for best performance requires `ripgrep`
      -- - `<Leader>fg` - find inside files; requires `ripgrep`
      -- - `<Leader>fh` - find help tag
      -- - `<Leader>fr` - resume latest picker
      -- - `<Leader>fv` - all visited paths; requires 'mini.visits'
      --
      -- All these use 'mini.pick'. See `:h MiniPick-overview` for an overview.
      local pick_added_hunks_buf = '<Cmd>Pick git_hunks path="%" scope="staged"<CR>'
      local pick_workspace_symbols_live = '<Cmd>Pick lsp scope="workspace_symbol_live"<CR>'

      nmap_leader('f/', '<Cmd>Pick history scope="/"<CR>',            '"/" history')
      nmap_leader('f:', '<Cmd>Pick history scope=":"<CR>',            '":" history')
      nmap_leader('fa', '<Cmd>Pick git_hunks scope="staged"<CR>',     'Added hunks (all)')
      nmap_leader('fA', pick_added_hunks_buf,                         'Added hunks (buf)')
      nmap_leader('fb', '<Cmd>Pick buffers<CR>',                      'Buffers')
      nmap_leader('fc', '<Cmd>Pick git_commits<CR>',                  'Commits (all)')
      nmap_leader('fC', '<Cmd>Pick git_commits path="%"<CR>',         'Commits (buf)')
      nmap_leader('fd', '<Cmd>Pick diagnostic scope="all"<CR>',       'Diagnostic workspace')
      nmap_leader('fD', '<Cmd>Pick diagnostic scope="current"<CR>',   'Diagnostic buffer')
      nmap_leader('ff', '<Cmd>Pick files<CR>',                        'Files')
      nmap_leader('fg', '<Cmd>Pick grep_live<CR>',                    'Grep live')
      nmap_leader('fG', '<Cmd>Pick grep pattern="<cword>"<CR>',       'Grep current word')
      nmap_leader('fh', '<Cmd>Pick help<CR>',                         'Help tags')
      nmap_leader('fH', '<Cmd>Pick hl_groups<CR>',                    'Highlight groups')
      nmap_leader('fl', '<Cmd>Pick buf_lines scope="all"<CR>',        'Lines (all)')
      nmap_leader('fL', '<Cmd>Pick buf_lines scope="current"<CR>',    'Lines (buf)')
      nmap_leader('fm', '<Cmd>Pick git_hunks<CR>',                    'Modified hunks (all)')
      nmap_leader('fM', '<Cmd>Pick git_hunks path="%"<CR>',           'Modified hunks (buf)')
      nmap_leader('fr', '<Cmd>Pick resume<CR>',                       'Resume')
      nmap_leader('fR', '<Cmd>Pick lsp scope="references"<CR>',       'References (LSP)')
      nmap_leader('fs', pick_workspace_symbols_live,                  'Symbols workspace (live)')
      nmap_leader('fS', '<Cmd>Pick lsp scope="document_symbol"<CR>',  'Symbols document')
      nmap_leader('fv', '<Cmd>Pick visit_paths cwd=""<CR>',           'Visit paths (all)')
      nmap_leader('fV', '<Cmd>Pick visit_paths<CR>',                  'Visit paths (cwd)')

      -- g is for 'Git'. Common usage:
      -- - `<Leader>gs` - show information at cursor
      -- - `<Leader>go` - toggle 'mini.diff' overlay to show in-buffer unstaged changes
      -- - `<Leader>gd` - show unstaged changes as a patch in separate tabpage
      -- - `<Leader>gL` - show Git log of current file
      local git_log_cmd = [[Git log --pretty=format:\%h\ \%as\ │\ \%s --topo-order]]
      local git_log_buf_cmd = git_log_cmd .. ' --follow -- %'

      nmap_leader('ga', '<Cmd>Git diff --cached<CR>',             'Added diff')
      nmap_leader('gA', '<Cmd>Git diff --cached -- %<CR>',        'Added diff buffer')
      nmap_leader('gc', '<Cmd>Git commit<CR>',                    'Commit')
      nmap_leader('gC', '<Cmd>Git commit --amend<CR>',            'Commit amend')
      nmap_leader('gd', '<Cmd>Git diff<CR>',                      'Diff')
      nmap_leader('gD', '<Cmd>Git diff -- %<CR>',                 'Diff buffer')
      nmap_leader('gl', '<Cmd>' .. git_log_cmd .. '<CR>',         'Log')
      nmap_leader('gL', '<Cmd>' .. git_log_buf_cmd .. '<CR>',     'Log buffer')
      nmap_leader('go', '<Cmd>lua MiniDiff.toggle_overlay()<CR>', 'Toggle overlay')
      nmap_leader('gs', '<Cmd>lua MiniGit.show_at_cursor()<CR>',  'Show at cursor')

      xmap_leader('gs', '<Cmd>lua MiniGit.show_at_cursor()<CR>', 'Show at selection')

      -- l is for 'Language'. Common usage:
      -- - `<Leader>ld` - show more diagnostic details in a floating window
      -- - `<Leader>lr` - perform rename via LSP
      -- - `<Leader>ls` - navigate to source definition of symbol under cursor
      --
      -- NOTE: most LSP mappings represent a more structured way of replacing built-in
      -- LSP mappings (like `:h gra` and others). This is needed because `gr` is mapped
      -- by an "replace" operator in 'mini.operators' (which is more commonly used).
      local formatting_cmd = '<Cmd>lua require("conform").format({lsp_fallback=true})<CR>'

      nmap_leader('la', '<Cmd>lua vim.lsp.buf.code_action()<CR>',     'Actions')
      nmap_leader('ld', '<Cmd>lua vim.diagnostic.open_float()<CR>',   'Diagnostic popup')
      nmap_leader('lf', formatting_cmd,                               'Format')
      nmap_leader('li', '<Cmd>lua vim.lsp.buf.implementation()<CR>',  'Implementation')
      nmap_leader('lh', '<Cmd>lua vim.lsp.buf.hover()<CR>',           'Hover')
      nmap_leader('lr', '<Cmd>lua vim.lsp.buf.rename()<CR>',          'Rename')
      nmap_leader('lR', '<Cmd>lua vim.lsp.buf.references()<CR>',      'References')
      nmap_leader('ls', '<Cmd>lua vim.lsp.buf.definition()<CR>',      'Source definition')
      nmap_leader('lt', '<Cmd>lua vim.lsp.buf.type_definition()<CR>', 'Type definition')

      xmap_leader('lf', formatting_cmd, 'Format selection')

      -- m is for 'Map'. Common usage:
      -- - `<Leader>mt` - toggle map from 'mini.map' (closed by default)
      -- - `<Leader>mf` - focus on the map for fast navigation
      -- - `<Leader>ms` - change map's side (if it covers something underneath)
      nmap_leader('mf', '<Cmd>lua MiniMap.toggle_focus()<CR>', 'Focus (toggle)')
      nmap_leader('mr', '<Cmd>lua MiniMap.refresh()<CR>',      'Refresh')
      nmap_leader('ms', '<Cmd>lua MiniMap.toggle_side()<CR>',  'Side (toggle)')
      nmap_leader('mt', '<Cmd>lua MiniMap.toggle()<CR>',       'Toggle')

      -- o is for 'Other'. Common usage:
      -- - `<Leader>oz` - toggle between "zoomed" and regular view of current buffer
      nmap_leader('or', '<Cmd>lua MiniMisc.resize_window()<CR>', 'Resize to default width')
      nmap_leader('ot', '<Cmd>lua MiniTrailspace.trim()<CR>',    'Trim trailspace')
      nmap_leader('oz', '<Cmd>lua MiniMisc.zoom()<CR>',          'Zoom toggle')

      -- s is for 'Session'. Common usage:
      -- - `<Leader>sn` - start new session
      -- - `<Leader>sr` - read previously started session
      -- - `<Leader>sd` - delete previously started session
      local session_new = 'MiniSessions.write(vim.fn.input("Session name: "))'

      nmap_leader('sd', '<Cmd>lua MiniSessions.select("delete")<CR>', 'Delete')
      nmap_leader('sn', '<Cmd>lua ' .. session_new .. '<CR>',         'New')
      nmap_leader('sr', '<Cmd>lua MiniSessions.select("read")<CR>',   'Read')
      nmap_leader('sw', '<Cmd>lua MiniSessions.write()<CR>',          'Write current')

      -- t is for 'Terminal'
      nmap_leader('tT', '<Cmd>horizontal term<CR>', 'Terminal (horizontal)')
      nmap_leader('tt', '<Cmd>vertical term<CR>',   'Terminal (vertical)')
      -- exit insert mode in terminal with ESC
      vim.keymap.set('t', '<Esc>', [[<C-\><C-n>]])

      -- v is for 'Visits'. Common usage:
      -- - `<Leader>vv` - add    "core" label to current file.
      -- - `<Leader>vV` - remove "core" label to current file.
      -- - `<Leader>vc` - pick among all files with "core" label.
      local make_pick_core = function(cwd, desc)
        return function()
          local sort_latest = MiniVisits.gen_sort.default({ recency_weight = 1 })
          local local_opts = { cwd = cwd, filter = 'core', sort = sort_latest }
          MiniExtra.pickers.visit_paths(local_opts, { source = { name = desc } })
        end
      end

      nmap_leader('vc', make_pick_core("",  'Core visits (all)'),       'Core visits (all)')
      nmap_leader('vC', make_pick_core(nil, 'Core visits (cwd)'),       'Core visits (cwd)')
      nmap_leader('vv', '<Cmd>lua MiniVisits.add_label("core")<CR>',    'Add "core" label')
      nmap_leader('vV', '<Cmd>lua MiniVisits.remove_label("core")<CR>', 'Remove "core" label')
      nmap_leader('vl', '<Cmd>lua MiniVisits.add_label()<CR>',          'Add label')
      nmap_leader('vL', '<Cmd>lua MiniVisits.remove_label()<CR>',       'Remove label')

      -- [[ Autocommands ]]
      local gr = vim.api.nvim_create_augroup('custom-config', {})
      new_autocmd = function(event, pattern, callback, desc)
        local opts = { group = gr, pattern = pattern, callback = callback, desc = desc }
        vim.api.nvim_create_autocmd(event, opts)
      end

      -- Don't auto-wrap comments and don't insert comment leader after hitting 'o'.
      -- Do on `FileType` to always override these changes from filetype plugins.
      local f = function() vim.cmd('setlocal formatoptions-=c formatoptions-=o') end
      new_autocmd('FileType', nil, f, "Proper 'formatoptions'")

      -- There are other autocommands created by 'mini.basics'.

      -- [[ Diagnostics ]]
      -- Neovim has built-in support for showing diagnostic messages. This configures
      -- a more conservative display while still being useful.
      -- See `:h vim.diagnostic` and `:h vim.diagnostic.config()`.
      local diagnostic_opts = {
        -- Show signs on top of any other sign, but only for warnings and errors
        signs = { priority = 9999, severity = { min = 'WARN', max = 'ERROR' } },

        -- Show all diagnostics as underline (for their messages type `<Leader>ld`)
        underline = { severity = { min = 'HINT', max = 'ERROR' } },

        -- Show more details immediately for errors on the current line
        virtual_lines = false,
        virtual_text = {
          current_line = true,
          severity = { min = 'ERROR', max = 'ERROR' },
        },

        -- Don't update diagnostics when typing
        update_in_insert = false,
      }

      -- Use `vim.schedule()` to avoid sourcing `vim.diagnostic` on startup
      vim.schedule(function() vim.diagnostic.config(diagnostic_opts) end)
      -- stylua: ignore end

      -- [[ Plugins ]]
    '';

    plugins = with pkgs.vimPlugins; [
      {
        plugin = mini-deps;
        type = "lua";
        config = /* lua */ ''
          -- Plugin Manager. All plugins are installed by Nix, only required for `now` and `later`.
          -- These could be replaced with immediate function calls and `vim.schedule`,
          -- but MiniDeps also handles errors gracefully, caches error messages and displays them later.
          require('mini.deps').setup()

          -- Helpers for installing/adding plugins in two stages
          local now, later = MiniDeps.now, MiniDeps.later

          -- Some plugins and 'mini.nvim' modules only need setup during startup if Neovim
          -- is started like `nvim -- path/to/file`, otherwise delaying setup is fine
          local now_if_args = vim.fn.argc(-1) > 0 and now or later
        '';
      }
      {
        plugin = mini-basics;
        type = "lua";
        config = /* lua */ ''
          -- Common configuration presets. Example usage:
          -- - `<C-s>` in Insert mode - save and go to Normal mode
          -- - `go` / `gO` - insert empty line before/after in Normal mode
          -- - `gy` / `gp` - copy / paste from system clipboard
          -- - `\` + key - toggle common options. Like `\h` toggles highlighting search.
          -- - `<C-hjkl>` (four combos) - navigate between windows.
          -- - `<M-hjkl>` in Insert/Command mode - navigate in that mode.
          --
          -- See also:
          -- - `:h MiniBasics.config.options` - list of adjusted options
          -- - `:h MiniBasics.config.mappings` - list of created mappings
          -- - `:h MiniBasics.config.autocommands` - list of created autocommands
          now(function()
            require('mini.basics').setup({
              -- Manage options in 'plugin/10_options.lua' for didactic purposes
              options = { basic = false },
              mappings = {
                -- Create `<C-hjkl>` mappings for window navigation
                windows = true,
                -- Create `<M-hjkl>` mappings for navigation in Insert and Command modes
                move_with_alt = true,
              },
            })
          end)
        '';
      }
      {
        plugin = mini-icons;
        type = "lua";
        config = /* lua */ ''
          -- Icon provider. Usually no need to use manually. It is used by plugins like
          -- 'mini.pick', 'mini.files', 'mini.statusline', and others.
          now(function()
            require('mini.icons').setup({style = 'glyph'})
          end)
        '';
      }
      {
        plugin = mini-misc;
        type = "lua";
        config = /* lua */ ''
          -- Miscellaneous small but useful functions. Example usage:
          -- - `<Leader>oz` - toggle between "zoomed" and regular view of current buffer
          -- - `<Leader>or` - resize window to its "editable width"
          -- - `:lua put_text(vim.lsp.get_clients())` - put output of a function below
          --   cursor in current buffer. Useful for a detailed exploration.
          -- - `:lua put(MiniMisc.stat_summary(MiniMisc.bench_time(f, 100)))` - run
          --   function `f` 100 times and report statistical summary of execution times
          --
          -- Uses `now()` for `setup_xxx()` to work when started like `nvim -- path/to/file`
          now_if_args(function()
            -- Makes `:h MiniMisc.put()` and `:h MiniMisc.put_text()` public
            require('mini.misc').setup()

            -- Change current working directory based on the current file path. It
            -- searches up the file tree until the first root marker ('.git' or 'Makefile')
            -- and sets their parent directory as a current directory.
            -- This is helpful when simultaneously dealing with files from several projects.
            MiniMisc.setup_auto_root()

            -- Restore latest cursor position on file open
            MiniMisc.setup_restore_cursor()

            -- Synchronize terminal emulator background with Neovim's background to remove
            -- possibly different color padding around Neovim instance
            MiniMisc.setup_termbg_sync()
          end)
        '';
      }
      {
        plugin = mini-notify;
        type = "lua";
        config = /* lua */ ''
          -- Notifications provider. Shows all kinds of notifications in the upper right
          -- corner (by default). Example usage:
          -- - `:h vim.notify()` - show notification (hides automatically)
          -- - `<Leader>en` - show notification history
          --
          -- See also:
          -- - `:h MiniNotify.config` for some of common configuration examples.
          now(function() require('mini.notify').setup() end)
        '';
      }
      {
        plugin = mini-sessions;
        type = "lua";
        config = /* lua */ ''
          -- Session management. A thin wrapper around `:h mksession` that consistently
          -- manages session files. Example usage:
          -- - `<Leader>sn` - start new session
          -- - `<Leader>sr` - read previously started session
          -- - `<Leader>sd` - delete previously started session
          now(function() require('mini.sessions').setup() end)
        '';
      }
      # {
      #   plugin = mini-starter;
      #   type = "lua";
      #   config = /* lua */ ''
      #     -- Start screen. This is what is shown when you open Neovim like `nvim`.
      #     -- Example usage:
      #     -- - Type prefix keys to limit available candidates
      #     -- - Navigate down/up with `<C-n>` and `<C-p>`
      #     -- - Press `<CR>` to select an entry
      #     --
      #     -- See also:
      #     -- - `:h MiniStarter-example-config` - non-default config examples
      #     -- - `:h MiniStarter-lifecycle` - how to work with Starter buffer
      #     now(function() require('mini.starter').setup() end)
      #   '';
      # }
      {
        plugin = mini-statusline;
        type = "lua";
        config = /* lua */ ''
          -- Statusline. Sets `:h 'statusline'` to show more info in a line below window.
          -- Example usage:
          -- - Left most section indicates current mode (text + highlighting).
          -- - Second from left section shows "developer info": Git, diff, diagnostics, LSP.
          -- - Center section shows the name of displayed buffer.
          -- - Second to right section shows more buffer info.
          -- - Right most section shows current cursor coordinates and search results.
          --
          -- See also:
          -- - `:h MiniStatusline-example-content` - example of default content. Use it to
          --   configure a custom statusline by setting `config.content.active` function.
          now(function() require('mini.statusline').setup() end)
        '';
      }
      {
        plugin = mini-tabline;
        type = "lua";
        config = /* lua */ ''
          -- Tabline. Sets `:h 'tabline'` to show all listed buffers in a line at the top.
          -- Buffers are ordered as they were created. Navigate with `[b` and `]b`.
          now(function() require('mini.tabline').setup() end)
        '';
      }
      {
        plugin = mini-extra;
        type = "lua";
        config = /* lua */ ''
          -- Extra 'mini.nvim' functionality.
          --
          -- See also:
          -- - `:h MiniExtra.pickers` - pickers. Most are mapped in `<Leader>f` group.
          --   Calling `setup()` makes 'mini.pick' respect 'mini.extra' pickers.
          -- - `:h MiniExtra.gen_ai_spec` - 'mini.ai' textobject specifications
          -- - `:h MiniExtra.gen_highlighter` - 'mini.hipatterns' highlighters
          later(function() require('mini.extra').setup() end)
        '';
      }
      {
        plugin = mini-ai;
        type = "lua";
        config = /* lua */ ''
          -- Extend and create a/i textobjects, like `:h a(`, `:h a'`, and more).
          -- Contains not only `a` and `i` type of textobjects, but also their "next" and
          -- "last" variants that will explicitly search for textobjects after and before
          -- cursor. Example usage:
          -- - `ci)` - *c*hange *i*inside parenthesis (`)`)
          -- - `di(` - *d*elete *i*inside padded parenthesis (`(`)
          -- - `yaq` - *y*ank *a*round *q*uote (any of "", \'\', or ``)
          -- - `vif` - *v*isually select *i*inside *f*unction call
          -- - `cina` - *c*hange *i*nside *n*ext *a*rgument
          -- - `valaala` - *v*isually select *a*round *l*ast (i.e. previous) *a*rgument
          --   and then again reselect *a*round new *l*ast *a*rgument
          --
          -- See also:
          -- - `:h text-objects` - general info about what textobjects are
          -- - `:h MiniAi-builtin-textobjects` - list of all supported textobjects
          -- - `:h MiniAi-textobject-specification` - examples of custom textobjects
          later(function()
            local ai = require('mini.ai')
            ai.setup({
              -- 'mini.ai' can be extended with custom textobjects
              custom_textobjects = {
                -- Make `aB` / `iB` act on around/inside whole *b*uffer
                B = MiniExtra.gen_ai_spec.buffer(),
                -- For more complicated textobjects that require structural awareness,
                -- use tree-sitter. This example makes `aF`/`iF` mean around/inside function
                -- definition (not call). See `:h MiniAi.gen_spec.treesitter()` for details.
                F = ai.gen_spec.treesitter({ a = '@function.outer', i = '@function.inner' }),
              },

              -- 'mini.ai' by default mostly mimics built-in search behavior: first try
              -- to find textobject covering cursor, then try to find to the right.
              -- Although this works in most cases, some are confusing. It is more robust to
              -- always try to search only covering textobject and explicitly ask to search
              -- for next (`an`/`in`) or last (`al`/`il`).
              -- Try this. If you don't like it - delete next line and this comment.
              search_method = 'cover',
            })
          end)
        '';
      }
      {
        plugin = mini-align;
        type = "lua";
        config = /* lua */ ''
          -- Align text interactively. Example usage:
          -- - `gaip,` - `ga` (align operator) *i*nside *p*aragraph by comma
          -- - `gAip` - start interactive alignment on the paragraph. Choose how to
          --   split, justify, and merge string parts. Press `<CR>` to make it permanent,
          --   press `<Esc>` to go back to initial state.
          --
          -- See also:
          -- - `:h MiniAlign-example` - hands-on list of examples to practice aligning
          -- - `:h MiniAlign.gen_step` - list of support step customizations
          -- - `:h MiniAlign-algorithm` - how alignment is done on algorithmic level
          later(function() require('mini.align').setup() end)
        '';
      }
      # {
      #   plugin = mini-animate;
      #   type = "lua";
      #   config = /* lua */ ''
      #     -- Animate common Neovim actions. Like cursor movement, scroll, window resize,
      #     -- window open, window close. Animations are done based on Neovim events and
      #     -- don't require custom mappings.
      #     later(function() require('mini.animate').setup() end)
      #   '';
      # }
      {
        plugin = mini-bracketed;
        type = "lua";
        config = /* lua */ ''
          -- Go forward/backward with square brackets. Implements consistent sets of mappings
          -- for selected targets (like buffers, diagnostic, quickfix list entries, etc.).
          -- Example usage:
          -- - `]b` - go to next buffer
          -- - `[j` - go to previous jump inside current buffer
          -- - `[Q` - go to first entry of quickfix list
          -- - `]X` - go to last conflict marker in a buffer
          --
          -- See also:
          -- - `:h MiniBracketed` - overall mapping design and list of targets
          later(function() require('mini.bracketed').setup() end)
        '';
      }
      {
        plugin = mini-bufremove;
        type = "lua";
        config = /* lua */ ''
          -- Remove buffers. Opened files occupy space in tabline and buffer picker.
          -- When not needed, they can be removed. Example usage:
          -- - `<Leader>bw` - completely wipeout current buffer (see `:h :bwipeout`)
          -- - `<Leader>bW` - completely wipeout current buffer even if it has changes
          -- - `<Leader>bd` - delete current buffer (see `:h :bdelete`)
          later(function() require('mini.bufremove').setup() end)
        '';
      }
      {
        plugin = mini-clue;
        type = "lua";
        config = /* lua */ ''
          -- Show next key clues in a bottom right window. Requires explicit opt-in for
          -- keys that act as clue trigger. Example usage:
          -- - Press `<Leader>` and wait for 1 second. A window with information about
          --   next available keys should appear.
          -- - Press one of the listed keys. Window updates immediately to show information
          --   about new next available keys. You can press `<BS>` to go back in key sequence.
          -- - Press keys until they resolve into some mapping.
          --
          -- Note: it is designed to work in buffers for normal files. It doesn't work in
          -- special buffers (like for 'mini.starter' or 'mini.files') to not conflict
          -- with its local mappings.
          --
          -- See also:
          -- - `:h MiniClue-examples` - examples of common setups
          -- - `:h MiniClue.ensure_buf_triggers()` - use it to enable triggers in buffer
          -- - `:h MiniClue.set_mapping_desc()` - change mapping description not from config
          later(function()
            local miniclue = require('mini.clue')
            -- stylua: ignore
            miniclue.setup({
              -- Define which clues to show. By default shows only clues for custom mappings
              -- (uses `desc` field from the mapping; takes precedence over custom clue).
              clues = {
                -- This is defined in 'plugin/20_keymaps.lua' with Leader group descriptions
                leader_group_clues,
                miniclue.gen_clues.builtin_completion(),
                miniclue.gen_clues.g(),
                miniclue.gen_clues.marks(),
                miniclue.gen_clues.registers(),
                -- This creates a submode for window resize mappings. Try the following:
                -- - Press `<C-w>s` to make a window split.
                -- - Press `<C-w>+` to increase height. Clue window still shows clues as if
                --   `<C-w>` is pressed again. Keep pressing just `+` to increase height.
                --   Try pressing `-` to decrease height.
                -- - Stop submode either by `<Esc>` or by any key that is not in submode.
                miniclue.gen_clues.windows({ submode_resize = true }),
                miniclue.gen_clues.z(),
              },
              -- Explicitly opt-in for set of common keys to trigger clue window
              triggers = {
                { mode = 'n', keys = '<Leader>' }, -- Leader triggers
                { mode = 'x', keys = '<Leader>' },
                { mode = 'n', keys = '\\' },       -- mini.basics
                { mode = 'n', keys = '[' },        -- mini.bracketed
                { mode = 'n', keys = ']' },
                { mode = 'x', keys = '[' },
                { mode = 'x', keys = ']' },
                { mode = 'i', keys = '<C-x>' },    -- Built-in completion
                { mode = 'n', keys = 'g' },        -- `g` key
                { mode = 'x', keys = 'g' },
                { mode = 'n', keys = "'" },        -- Marks
                { mode = 'n', keys = '`' },
                { mode = 'x', keys = "'" },
                { mode = 'x', keys = '`' },
                { mode = 'n', keys = '"' },        -- Registers
                { mode = 'x', keys = '"' },
                { mode = 'i', keys = '<C-r>' },
                { mode = 'c', keys = '<C-r>' },
                { mode = 'n', keys = '<C-w>' },    -- Window commands
                { mode = 'n', keys = 'z' },        -- `z` key
                { mode = 'x', keys = 'z' },
              },
            })
          end)
        '';
      }
      {
        plugin = mini-comment;
        type = "lua";
        config = /* lua */ ''
          -- Comment lines. Provides functionality to work with commented lines.
          -- Uses `:h 'commentstring'` option to infer comment structure.
          -- Example usage:
          -- - `gcip` - toggle comment (`gc`) *i*inside *p*aragraph
          -- - `vapgc` - *v*isually select *a*round *p*aragraph and toggle comment (`gc`)
          -- - `gcgc` - uncomment (`gc`, operator) comment block at cursor (`gc`, textobject)
          --
          -- The built-in `:h commenting` is based on 'mini.comment'. Yet this module is
          -- still enabled as it provides more customization opportunities.
          later(function() require('mini.comment').setup() end)
        '';
      }
      {
        plugin = mini-completion;
        type = "lua";
        config = /* lua */ ''
          -- Completion and signature help. Implements async "two stage" autocompletion:
          -- - Based on attached LSP servers that support completion.
          -- - Fallback (based on built-in keyword completion) if there is no LSP candidates.
          --
          -- Example usage in Insert mode with attached LSP:
          -- - Start typing text that should be recognized by LSP (like variable name).
          -- - After 100ms a popup menu with candidates appears.
          -- - Press `<Tab>` / `<S-Tab>` to navigate down/up the list. These are set up
          --   in 'mini.keymap'. You can also use `<C-n>` / `<C-p>`.
          -- - During navigation there is an info window to the right showing extra info
          --   that the LSP server can provide about the candidate. It appears after the
          --   candidate stays selected for 100ms. Use `<C-f>` / `<C-b>` to scroll it.
          -- - Navigating to an entry also changes buffer text. If you are happy with it,
          --   keep typing after it. To discard completion completely, press `<C-e>`.
          -- - After pressing special trigger(s), usually `(`, a window appears that shows
          --   the signature of the current function/method. It gets updated as you type
          --   showing the currently active parameter.
          --
          -- Example usage in Insert mode without an attached LSP or in places not
          -- supported by the LSP (like comments):
          -- - Start typing a word that is present in current or opened buffers.
          -- - After 100ms popup menu with candidates appears.
          -- - Navigate with `<Tab>` / `<S-Tab>` or `<C-n>` / `<C-p>`. This also updates
          --   buffer text. If happy with choice, keep typing. Stop with `<C-e>`.
          --
          -- It also works with snippet candidates provided by LSP server. Best experience
          -- when paired with 'mini.snippets' (which is set up in this file).
          later(function()
            -- Customize post-processing of LSP responses for a better user experience.
            -- Don't show 'Text' suggestions (usually noisy) and show snippets last.
            local process_items_opts = { kind_priority = { Text = -1, Snippet = 99 } }
            local process_items = function(items, base)
              return MiniCompletion.default_process_items(items, base, process_items_opts)
            end
            require('mini.completion').setup({
              lsp_completion = {
                -- Without this config autocompletion is set up through `:h 'completefunc'`.
                -- Although not needed, setting up through `:h 'omnifunc'` is cleaner
                -- (sets up only when needed) and makes it possible to use `<C-u>`.
                source_func = 'omnifunc',
                auto_setup = false,
                process_items = process_items,
              },
            })

            -- Set 'omnifunc' for LSP completion only when needed.
            local on_attach = function(ev)
              vim.bo[ev.buf].omnifunc = 'v:lua.MiniCompletion.completefunc_lsp'
            end
            new_autocmd('LspAttach', nil, on_attach, "Set 'omnifunc'")

            -- Advertise to servers that Neovim now supports certain set of completion and
            -- signature features through 'mini.completion'.
            vim.lsp.config('*', { capabilities = MiniCompletion.get_lsp_capabilities() })
          end)
        '';
      }
      {
        plugin = mini-cursorword;
        type = "lua";
        config = /* lua */ ''
          -- Autohighlight word under cursor with a customizable delay.
          -- Word boundaries are defined based on `:h 'iskeyword'` option.
          later(function() require('mini.cursorword').setup() end)
        '';
      }
      {
        plugin = mini-diff;
        type = "lua";
        config = /* lua */ ''
          -- Work with diff hunks that represent the difference between the buffer text and
          -- some reference text set by a source. Default source uses text from Git index.
          -- Also provides summary info used in developer section of 'mini.statusline'.
          -- Example usage:
          -- - `ghip` - apply hunks (`gh`) within *i*nside *p*aragraph
          -- - `gHG` - reset hunks (`gH`) from cursor until end of buffer (`G`)
          -- - `ghgh` - apply (`gh`) hunk at cursor (`gh`)
          -- - `gHgh` - reset (`gH`) hunk at cursor (`gh`)
          -- - `<Leader>go` - toggle overlay
          --
          -- See also:
          -- - `:h MiniDiff-overview` - overview of how module works
          -- - `:h MiniDiff-diff-summary` - available summary information
          -- - `:h MiniDiff.gen_source` - available built-in sources
          later(function() require('mini.diff').setup() end)
        '';
      }
      {
        plugin = mini-files;
        type = "lua";
        config = /* lua */ ''
          -- Navigate and manipulate file system
          --
          -- Navigation is done using column view (Miller columns) to display nested
          -- directories, they are displayed in floating windows in top left corner.
          --
          -- Manipulate files and directories by editing text as regular buffers.
          --
          -- Example usage:
          -- - `<Leader>ed` - open current working directory
          -- - `<Leader>ef` - open directory of current file (needs to be present on disk)
          --
          -- Basic navigation:
          -- - `l` - go in entry at cursor: navigate into directory or open file
          -- - `h` - go out of focused directory
          -- - Navigate window as any regular buffer
          -- - Press `g?` inside explorer to see more mappings
          --
          -- Basic manipulation:
          -- - After any following action, press `=` in Normal mode to synchronize, read
          --   carefully about actions, press `y` or `<CR>` to confirm
          -- - New entry: press `o` and type its name; end with `/` to create directory
          -- - Rename: press `C` and type new name
          -- - Delete: type `dd`
          -- - Move/copy: type `dd`/`yy`, navigate to target directory, press `p`
          --
          -- See also:
          -- - `:h MiniFiles-navigation` - more details about how to navigate
          -- - `:h MiniFiles-manipulation` - more details about how to manipulate
          -- - `:h MiniFiles-examples` - examples of common setups
          later(function()
            -- Enable directory/file preview
            require('mini.files').setup({ windows = { preview = true } })

            -- Add common bookmarks for every explorer. Example usage inside explorer:
            -- - `'c` to navigate into your config directory
            -- - `g?` to see available bookmarks
            local add_marks = function()
              MiniFiles.set_bookmark('c', vim.fn.stdpath('config'), { desc = 'Config' })
              MiniFiles.set_bookmark('w', vim.fn.getcwd, { desc = 'Working directory' })
            end
            new_autocmd('User', 'MiniFilesExplorerOpen', add_marks, 'Add bookmarks')
          end)
        '';
      }
      {
        plugin = mini-git;
        type = "lua";
        config = /* lua */ ''
          -- Git integration for more straightforward Git actions based on Neovim's state.
          -- It is not meant as a fully featured Git client, only to provide helpers that
          -- integrate better with Neovim. Example usage:
          -- - `<Leader>gs` - show information at cursor
          -- - `<Leader>gd` - show unstaged changes as a patch in separate tabpage
          -- - `<Leader>gL` - show Git log of current file
          -- - `:Git help git` - show output of `git help git` inside Neovim
          --
          -- See also:
          -- - `:h MiniGit-examples` - examples of common setups
          -- - `:h :Git` - more details about `:Git` user command
          -- - `:h MiniGit.show_at_cursor()` - what information at cursor is shown
          later(function() require('mini.git').setup() end)
        '';
      }
      {
        plugin = mini-hipatterns;
        type = "lua";
        config = /* lua */ ''
          -- Highlight patterns in text. Like `TODO`/`NOTE` or color hex codes.
          -- Example usage:
          -- - `:Pick hipatterns` - pick among all highlighted patterns
          --
          -- See also:
          -- - `:h MiniHipatterns-examples` - examples of common setups
          later(function()
            local hipatterns = require('mini.hipatterns')
            local hi_words = MiniExtra.gen_highlighter.words
            hipatterns.setup({
              highlighters = {
                -- Highlight a fixed set of common words. Will be highlighted in any place,
                -- not like "only in comments".
                fixme = hi_words({ 'FIXME', 'Fixme', 'fixme' }, 'MiniHipatternsFixme'),
                hack = hi_words({ 'HACK', 'Hack', 'hack' }, 'MiniHipatternsHack'),
                todo = hi_words({ 'TODO', 'Todo', 'todo' }, 'MiniHipatternsTodo'),
                note = hi_words({ 'NOTE', 'Note', 'note' }, 'MiniHipatternsNote'),

                -- Highlight hex color string (#aabbcc) with that color as a background
                hex_color = hipatterns.gen_highlighter.hex_color(),
              },
            })
          end)
        '';
      }
      {
        plugin = mini-indentscope;
        type = "lua";
        config = /* lua */ ''
          -- Visualize and work with indent scope. It visualizes indent scope "at cursor"
          -- with animated vertical line. Provides relevant motions and textobjects.
          -- Example usage:
          -- - `cii` - *c*hange *i*nside *i*ndent scope
          -- - `Vaiai` - *V*isually select *a*round *i*ndent scope and then again
          --   reselect *a*round new *i*indent scope
          -- - `[i` / `]i` - navigate to scope's top / bottom
          --
          -- See also:
          -- - `:h MiniIndentscope.gen_animation` - available animation rules
          later(function() require('mini.indentscope').setup() end)
        '';
      }
      # {
      #   plugin = mini-jump;
      #   type = "lua";
      #   config = /* lua */ ''
      #     -- Jump to next/previous single character. It implements "smarter `fFtT` keys"
      #     -- (see `:h f`) that work across multiple lines, start "jumping mode", and
      #     -- highlight all target matches. Example usage:
      #     -- - `fxff` - move *f*orward onto next character "x", then next, and next again
      #     -- - `dt)` - *d*elete *t*ill next closing parenthesis (`)`)
      #     later(function() require('mini.jump').setup() end)
      #   '';
      # }
      # {
      #   plugin = mini-jump2d;
      #   type = "lua";
      #   config = /* lua */ ''
      #     -- Jump within visible lines to pre-defined spots via iterative label filtering.
      #     -- Spots are computed by a configurable spotter function. Example usage:
      #     -- - Lock eyes on desired location to jump
      #     -- - `<CR>` - start jumping; this shows character labels over target spots
      #     -- - Type character that appears over desired location; number of target spots
      #     --   should be reduced
      #     -- - Keep typing labels until target spot is unique to perform the jump
      #     --
      #     -- See also:
      #     -- - `:h MiniJump2d.gen_spotter` - list of available spotters
      #     later(function() require('mini.jump2d').setup() end)
      #   '';
      # }
      {
        plugin = mini-keymap;
        type = "lua";
        config = /* lua */ ''
          -- Special key mappings. Provides helpers to map:
          -- - Multi-step actions. Apply action 1 if condition is met; else apply
          --   action 2 if condition is met; etc.
          -- - Combos. Sequence of keys where each acts immediately plus execute extra
          --   action if all are typed fast enough. Useful for Insert mode mappings to not
          --   introduce delay when typing mapping keys without intention to execute action.
          --
          -- See also:
          -- - `:h MiniKeymap-examples` - examples of common setups
          -- - `:h MiniKeymap.map_multistep()` - map multi-step action
          -- - `:h MiniKeymap.map_combo()` - map combo
          later(function()
            require('mini.keymap').setup()
            -- Navigate 'mini.completion' menu with `<Tab>` /  `<S-Tab>`
            MiniKeymap.map_multistep('i', '<Tab>', { 'pmenu_next' })
            MiniKeymap.map_multistep('i', '<S-Tab>', { 'pmenu_prev' })
            -- On `<CR>` try to accept current completion item, fall back to accounting
            -- for pairs from 'mini.pairs'
            MiniKeymap.map_multistep('i', '<CR>', { 'pmenu_accept', 'minipairs_cr' })
            -- On `<BS>` just try to account for pairs from 'mini.pairs'
            MiniKeymap.map_multistep('i', '<BS>', { 'minipairs_bs' })
          end)
        '';
      }
      {
        plugin = mini-map;
        type = "lua";
        config = /* lua */ ''
          -- Window with text overview. It is displayed on the right hand side. Can be used
          -- for quick overview and navigation. Hidden by default. Example usage:
          -- - `<Leader>mt` - toggle map window
          -- - `<Leader>mf` - focus on the map for fast navigation
          -- - `<Leader>ms` - change map's side (if it covers something underneath)
          --
          -- See also:
          -- - `:h MiniMap.gen_encode_symbols` - list of symbols to use for text encoding
          -- - `:h MiniMap.gen_integration` - list of integrations to show in the map
          --
          -- NOTE: Might introduce lag on very big buffers (10000+ lines)
          later(function()
            local map = require('mini.map')
            map.setup({
              -- Use Braille dots to encode text
              symbols = { encode = map.gen_encode_symbols.dot('4x2') },
              -- Show built-in search matches, 'mini.diff' hunks, and diagnostic entries
              integrations = {
                map.gen_integration.builtin_search(),
                map.gen_integration.diff(),
                map.gen_integration.diagnostic(),
              },
            })

            -- Map built-in navigation characters to force map refresh
            for _, key in ipairs({ 'n', 'N', '*', '#' }) do
              local rhs = key
                -- Also open enough folds when jumping to the next match
                .. 'zv'
                .. '<Cmd>lua MiniMap.refresh({}, { lines = false, scrollbar = false })<CR>'
              vim.keymap.set('n', key, rhs)
            end
          end)
        '';
      }
      {
        plugin = mini-move;
        type = "lua";
        config = /* lua */ ''
          -- Move any selection in any direction. Example usage in Normal mode:
          -- - `<M-j>`/`<M-k>` - move current line down / up
          -- - `<M-h>`/`<M-l>` - decrease / increase indent of current line
          --
          -- Example usage in Visual mode:
          -- - `<M-h>`/`<M-j>`/`<M-k>`/`<M-l>` - move selection left/down/up/right
          later(function() require('mini.move').setup() end)
        '';
      }
      # {
      #   plugin = mini-operators;
      #   type = "lua";
      #   config = /* lua */ ''
      #     -- Text edit operators. All operators have mappings for:
      #     -- - Regular operator (waits for motion/textobject to use)
      #     -- - Current line action (repeat second character of operator to activate)
      #     -- - Act on visual selection (type operator in Visual mode)
      #     --
      #     -- Example usage:
      #     -- - `griw` - replace (`gr`) *i*inside *w*ord
      #     -- - `gmm` - multiple/duplicate (`gm`) current line (extra `m`)
      #     -- - `vipgs` - *v*isually select *i*nside *p*aragraph and sort it (`gs`)
      #     -- - `gxiww.` - exchange (`gx`) *i*nside *w*ord with next word (`w` to navigate
      #     --   to it and `.` to repeat exchange operator)
      #     -- - `g==` - execute current line as Lua code and replace with its output.
      #     --   For example, typing `g==` over line `vim.lsp.get_clients()` shows
      #     --   information about all available LSP clients.
      #     --
      #     -- See also:
      #     -- - `:h MiniOperators-mappings` - overview of how mappings are created
      #     -- - `:h MiniOperators-overview` - overview of present operators
      #     later(function()
      #       require('mini.operators').setup()

      #       -- Create mappings for swapping adjacent arguments. Notes:
      #       -- - Relies on `a` argument textobject from 'mini.ai'.
      #       -- - It is not 100% reliable, but mostly works.
      #       -- - It overrides `:h (` and `:h )`.
      #       -- Explanation: `gx`-`ia`-`gx`-`ila` <=> exchange current and last argument
      #       -- Usage: when on `a` in `(aa, bb)` press `)` followed by `(`.
      #       vim.keymap.set('n', '(', 'gxiagxila', { remap = true, desc = 'Swap arg left' })
      #       vim.keymap.set('n', ')', 'gxiagxina', { remap = true, desc = 'Swap arg right' })
      #     end)
      #   '';
      # }
      {
        plugin = mini-pairs;
        type = "lua";
        config = /* lua */ ''
          -- Autopairs functionality. Insert pair when typing opening character and go over
          -- right character if it is already to cursor's right. Also provides mappings for
          -- `<CR>` and `<BS>` to perform extra actions when inside pair.
          -- Example usage in Insert mode:
          -- - `(` - insert "()" and put cursor between them
          -- - `)` when there is ")" to the right - jump over ")" without inserting new one
          -- - `<C-v>(` - always insert a single "(" literally. This is useful since
          --   'mini.pairs' doesn't provide particularly smart behavior, like auto balancing
          later(function()
            -- Create pairs not only in Insert, but also in Command line mode
            require('mini.pairs').setup({ modes = { command = true } })
          end)
        '';
      }
      {
        plugin = mini-pick;
        type = "lua";
        config = /* lua */ ''
          -- Pick anything with single window layout and fast matching. This is one of
          -- the main usability improvements as it powers a lot of "find things quickly"
          -- workflows. How to use a picker:
          -- - Start picker, usually with `:Pick <picker-name>` command. Like `:Pick files`.
          --   It shows a single window in the bottom left corner filled with possible items
          --   to choose from. Current item has special full line highlighting.
          --   At the top there is a current query used to filter+sort items.
          -- - Type characters (appear at top) to narrow down items. There is fuzzy matching:
          --   characters may not match one-by-one, but they should be in correct order.
          -- - Navigate down/up with `<C-n>`/`<C-p>`.
          -- - Press `<Tab>` to show item's preview. `<Tab>` again goes back to items.
          -- - Press `<S-Tab>` to show picker's info. `<S-Tab>` again goes back to items.
          -- - Press `<CR>` to choose an item. The exact action depends on the picker: `files`
          --   picker opens a selected file, `help` picker opens help page on selected tag.
          --   To close picker without choosing an item, press `<Esc>`.
          --
          -- Example usage:
          -- - `<Leader>ff` - *f*ind *f*iles; for best performance requires `ripgrep`
          -- - `<Leader>fg` - *f*ind inside files (a.k.a. "to *g*rep"); requires `ripgrep`
          -- - `<Leader>fh` - *f*ind *h*elp tag
          -- - `<Leader>fr` - *r*esume latest picker
          -- - `:h vim.ui.select()` - implemented with 'mini.pick'
          --
          -- See also:
          -- - `:h MiniPick-overview` - overview of picker functionality
          -- - `:h MiniPick-examples` - examples of common setups
          -- - `:h MiniPick.builtin` and `:h MiniExtra.pickers` - available pickers;
          --   Execute one either with Lua function, `:Pick <picker-name>` command, or
          --   one of `<Leader>f` mappings defined in 'plugin/20_keymaps.lua'
          later(function() require('mini.pick').setup() end)
        '';
      }
      # {
      #   plugin = mini-snippets;
      #   type = "lua";
      #   config = /* lua */ ''
      #     -- Manage and expand snippets (templates for a frequently used text).
      #     -- Typical workflow is to type snippet's (configurable) prefix and expand it
      #     -- into a snippet session.
      #     --
      #     -- How to manage snippets:
      #     -- - 'mini.snippets' itself doesn't come with preconfigured snippets. Instead there
      #     --   is a flexible system of how snippets are prepared before expanding.
      #     --   They can come from pre-defined path on disk, 'snippets/' directories inside
      #     --   config or plugins, defined inside `setup()` call directly.
      #     -- - This config, however, does come with snippet configuration:
      #     --     - 'snippets/global.json' is a file with global snippets that will be
      #     --       available in any buffer
      #     --     - 'after/snippets/lua.json' defines personal snippets for Lua language
      #     --     - 'friendly-snippets' plugin configured in 'plugin/40_plugins.lua' provides
      #     --       a collection of language snippets
      #     --
      #     -- How to expand a snippet in Insert mode:
      #     -- - If you know snippet's prefix, type it as a word and press `<C-j>`. Snippet's
      #     --   body should be inserted instead of the prefix.
      #     -- - If you don't remember snippet's prefix, type only part of it (or none at all)
      #     --   and press `<C-j>`. It should show picker with all snippets that have prefixes
      #     --   matching typed characters (or all snippets if none was typed).
      #     --   Choose one and its body should be inserted instead of previously typed text.
      #     --
      #     -- How to navigate during snippet session:
      #     -- - Snippets can contain tabstops - places for user to interactively adjust text.
      #     --   Each tabstop is highlighted depending on session progression - whether tabstop
      #     --   is current, was or was not visited. If tabstop doesn't yet have text, it is
      #     --   visualized with special "ghost" inline text: • and ∎ by default.
      #     -- - Type necessary text at current tabstop and navigate to next/previous one
      #     --   by pressing `<C-l>` / `<C-h>`.
      #     -- - Repeat previous step until you reach special final tabstop, usually denoted
      #     --   by ∎ symbol. If you spotted a mistake in an earlier tabstop, navigate to it
      #     --   and return back to the final tabstop.
      #     -- - To end a snippet session when at final tabstop, keep typing or go into
      #     --   Normal mode. To force end snippet session, press `<C-c>`.
      #     --
      #     -- See also:
      #     -- - `:h MiniSnippets-overview` - overview of how module works
      #     -- - `:h MiniSnippets-examples` - examples of common setups
      #     -- - `:h MiniSnippets-session` - details about snippet session
      #     -- - `:h MiniSnippets.gen_loader` - list of available loaders
      #     later(function()
      #       -- Define language patterns to work better with 'friendly-snippets'
      #       local latex_patterns = { 'latex/**/*.json', '**/latex.json' }
      #       local lang_patterns = {
      #         tex = latex_patterns,
      #         plaintex = latex_patterns,
      #         -- Recognize special injected language of markdown tree-sitter parser
      #         markdown_inline = { 'markdown.json' },
      #       }

      #       local snippets = require('mini.snippets')
      #       local config_path = vim.fn.stdpath('config')
      #       snippets.setup({
      #         snippets = {
      #           -- Always load 'snippets/global.json' from config directory
      #           snippets.gen_loader.from_file(config_path .. '/snippets/global.json'),
      #           -- Load from 'snippets/' directory of plugins, like 'friendly-snippets'
      #           snippets.gen_loader.from_lang({ lang_patterns = lang_patterns }),
      #         },
      #       })

      #       -- By default snippets available at cursor are not shown as candidates in
      #       -- 'mini.completion' menu. This requires a dedicated in-process LSP server
      #       -- that will provide them. To have that, uncomment next line (use `gcc`).
      #       -- MiniSnippets.start_lsp_server()
      #     end)
      #   '';
      # }
      {
        plugin = mini-splitjoin;
        type = "lua";
        config = /* lua */ ''
          -- Split and join arguments (regions inside brackets between allowed separators).
          -- It uses Lua patterns to find arguments, which means it works in comments and
          -- strings but can be not as accurate as tree-sitter based solutions.
          -- Each action can be configured with hooks (like add/remove trailing comma).
          -- Example usage:
          -- - `gS` - toggle between joined (all in one line) and split (each on a separate
          --   line and indented) arguments. It is dot-repeatable (see `:h .`).
          --
          -- See also:
          -- - `:h MiniSplitjoin.gen_hook` - list of available hooks
          later(function() require('mini.splitjoin').setup() end)
        '';
      }
      {
        plugin = mini-surround;
        type = "lua";
        config = /* lua */ ''
          -- Surround actions: add/delete/replace/find/highlight. Working with surroundings
          -- is surprisingly common: surround word with quotes, replace `)` with `]`, etc.
          -- This module comes with many built-in surroundings, each identified by a single
          -- character. It searches only for surrounding that covers cursor and comes with
          -- a special "next" / "last" versions of actions to search forward or backward
          -- (just like 'mini.ai'). All text editing actions are dot-repeatable (see `:h .`).
          --
          -- Example usage (this may feel intimidating at first, but after practice it
          -- becomes second nature during text editing):
          -- - `saiw)` - *s*urround *a*dd for *i*nside *w*ord parenthesis (`)`)
          -- - `sdf`   - *s*urround *d*elete *f*unction call (like `f(var)` -> `var`)
          -- - `srb[`  - *s*urround *r*eplace *b*racket (any of [], (), {}) with padded `[`
          -- - `sf*`   - *s*urround *f*ind right part of `*` pair (like bold in markdown)
          -- - `shf`   - *s*urround *h*ighlight current *f*unction call
          -- - `srn{{` - *s*urround *r*eplace *n*ext curly bracket `{` with padded `{`
          -- - `sdl'`  - *s*urround *d*elete *l*ast quote pair (`'`)
          -- - `vaWsa<Space>` - *v*isually select *a*round *W*ORD and *s*urround *a*dd
          --                    spaces (`<Space>`)
          --
          -- See also:
          -- - `:h MiniSurround-builtin-surroundings` - list of all supported surroundings
          -- - `:h MiniSurround-surrounding-specification` - examples of custom surroundings
          -- - `:h MiniSurround-vim-surround-config` - alternative set of action mappings
          later(function() require('mini.surround').setup() end)
        '';
      }
      {
        plugin = mini-trailspace;
        type = "lua";
        config = /* lua */ ''
          -- Highlight and remove trailspace. Temporarily stops highlighting in Insert mode
          -- to reduce noise when typing. Example usage:
          -- - `<Leader>ot` - trim all trailing whitespace in a buffer
          later(function() require('mini.trailspace').setup() end)
        '';
      }
      {
        plugin = mini-visits;
        type = "lua";
        config = /* lua */ ''
          -- Track and reuse file system visits. Every file/directory visit is persistently
          -- tracked on disk to later reuse: show in special frecency order, etc. It also
          -- supports adding labels to visited paths to quickly navigate between them.
          -- Example usage:
          -- - `<Leader>fv` - find across all visits
          -- - `<Leader>vv` / `<Leader>vV` - add/remove special "core" label to current file
          -- - `<Leader>vc` / `<Leader>vC` - show files with "core" label; all or added within
          --   current working directory
          --
          -- See also:
          -- - `:h MiniVisits-overview` - overview of how module works
          -- - `:h MiniVisits-examples` - examples of common setups
          later(function() require('mini.visits').setup() end)
        '';
      }
      {
        plugin = nvim-treesitter.withAllGrammars;
        type = "lua";
        config = /* lua */ ''
          vim.api.nvim_create_autocmd('FileType', {
            group = vim.api.nvim_create_augroup('treesitter.setup', {}),
            callback = function(args)
              local buf = args.buf
              local filetype = args.match

              -- checks if a parser exists for the current language
              local language = vim.treesitter.language.get_lang(filetype) or filetype
              if vim.treesitter.language.add(language) then
                -- enable folds
                vim.wo.foldexpr = 'v:lua.vim.treesitter.foldexpr()'
                vim.wo.foldmethod = 'expr'

                -- enable highlighting
                vim.treesitter.start(buf, language)

                -- enable indentation
                vim.bo[buf].indentexpr = "v:lua.require'nvim-treesitter'.indentexpr()"
              end
            end,
          })
        '';
      }
      {
        plugin = nvim-lspconfig;
        type = "lua";
        config = /* lua */ ''
                    -- [[ Language Servers ]]
                    -- lsp setup is managed via vim.lsp, but nvim-lspconfig provides the configurations
                    -- see https://github.com/neovim/nvim-lspconfig/tree/master/lsp for defaults

                    -- C
                    vim.lsp.enable('clangd')

                    -- Haskell
                    vim.lsp.enable('hls')

                    -- Julia
                    -- NOTE: this requires manual setup for now
                    --- LanguageServer.jl, SymbolServer.jl and StaticLint.jl can be installed with `julia` and `Pkg`:
                    --- ```sh
                    --- julia --project=~/.julia/environments/nvim-lspconfig -e 'using Pkg; Pkg.add("LanguageServer"); Pkg.add("SymbolServer"); Pkg.add("StaticLint")'
                    --- ```
                    --- where `~/.julia/environments/nvim-lspconfig` is the location where
                    --- the default configuration expects LanguageServer.jl, SymbolServer.jl and StaticLint.jl to be installed.
                    ---
                    --- To update an existing install, use the following command:
                    --- ```sh
                    --- julia --project=~/.julia/environments/nvim-lspconfig -e 'using Pkg; Pkg.update()'
                    --- ```
                    ---
                    --- Note: In order to have LanguageServer.jl pick up installed packages or dependencies in a
                    --- Julia project, you must make sure that the project is instantiated:
                    --- ```sh
                    --- julia --project=/path/to/my/project -e 'using Pkg; Pkg.instantiate()'
                    --- ```
                    ---
                    --- Note: The julia programming language searches for global environments within the `environments/`
                    --- folder of `$JULIA_DEPOT_PATH` entries. By default this simply `~/.julia/environments`
                    vim.lsp.enable('julials')

                    -- Lua
                    vim.lsp.enable('lua_ls')
                    vim.lsp.enable('stylua')

                    -- Markdown
                    vim.lsp.enable('markdown_oxide')
                    vim.lsp.enable('marksman')

                    -- Nginx
                    vim.lsp.enable('nginx_language_server')

                    -- Nix
                    vim.lsp.config('nixd', {
                      cmd = { "nixd" },
                        settings = {
                        nixd = {
                          nixpkgs = {
                            expr = "import <nixpkgs> { }",
                          },
                          formatting = {
                            command = { "nixfmt" },
                          },
                        },
          	          },
                    })
                    vim.lsp.enable('nixd')

                    -- Python
                    vim.lsp.enable('basedpyright')
                    vim.lsp.enable('ruff')

                    -- TeX
                    vim.lsp.enable('texlab')

                    -- Typst
                    vim.lsp.enable("tinymist")

                    -- YAML
                    vim.lsp.enable("yamlls")

                    -- Language Tool
                    vim.lsp.enable("ltex_plus")
                    -- TODO: setup languages, like https://github.com/DannyBronzino/nvim-lua-config/blob/a9992b64c7d7ea67e6f8666fa79887437cfff811/lua/plugins/lsp.lua#L114
        '';
      }
      {
        plugin = typst-preview-nvim;
        type = "lua";
        config = /* lua */ ''
          later(function()
            require 'typst-preview'.setup {
              -- Custom format string to open the output link provided with %s
              -- Example: open_cmd = 'firefox %s -P typst-preview --class typst-preview'
              open_cmd = nil,

              -- Custom port to open the preview server. Default is random.
              -- Example: port = 8000
              port = 0,

              -- Setting this to 'always' will invert black and white in the preview
              -- Setting this to 'auto' will invert depending if the browser has enable
              -- dark mode
              -- Setting this to '{"rest": "<option>","image": "<option>"}' will apply
              -- your choice of color inversion to images and everything else
              -- separately.
              invert_colors = 'never',

              -- Whether the preview will follow the cursor in the source file
              follow_cursor = true,

              -- Provide the path to binaries for dependencies.
              -- Setting this will skip the download of the binary by the plugin.
              -- Warning: Be aware that your version might be older than the one
              -- required.
              dependencies_bin = {
                ['tinymist'] = 'tinymist',
                ['websocat'] = 'websocat'
              },
            }
          end)
        '';
      }
      {
        plugin = conform-nvim;
        type = "lua";
        config = /* lua */ ''
          -- Programs dedicated to text formatting (a.k.a. formatters) are very useful.
          -- Neovim has built-in tools for text formatting (see `:h gq` and `:h 'formatprg'`).
          -- They can be used to configure external programs, but it might become tedious.
          --
          -- The 'stevearc/conform.nvim' plugin is a good and maintained solution for easier
          -- formatting setup.
          later(function()
            -- See also:
            -- - `:h Conform`
            -- - `:h conform-options`
            -- - `:h conform-formatters`
            require('conform').setup({
              -- Map of filetype to formatters
              -- Make sure that necessary CLI tool is available
              formatters_by_ft = { lua = { 'stylua' } },
            })
          end)
        '';
      }
      {
        plugin = hardtime-nvim;
        type = "lua";
        config = /* lua */ ''
          later(function()
            require("hardtime").setup()
          end)
        '';
      }
    ];
  };

  preservation.preserveAt.state-dir.directories = [
    ".local/share/nvim"
  ];
}
