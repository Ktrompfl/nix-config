-- This configuration is built on [waywall_generic_config](https://github.com/arjuncgore/waywall_generic_config/tree/1440).
-- See the respective README for more information.

-- ==== WAYWALL GENERIC CONFIG ====
local main = require("main")

local cfg = {
	-- ==== LOOKS ====
	resolution = { 2560, 1440 },

	bg_col = "#000000",
	toggle_bg_picture = false,
	text_col = "#FFFFFF",
	pie_chart_1 = "#EC6E4E",
	pie_chart_2 = "#46CE66",
	pie_chart_3 = "#E446C4",

	-- ==== ALTERNATIVE RESOLUTIONS ====
	thin_res = { 400, 1440 },
	wide_res = { 2560, 400 },
	tall_res = { 384, 16384 },

	-- ==== MIRRORS ====
	e_count = { enabled = true, x = 1500, y = 400, size = 5, colorkey = true, show_c = false },

	thin_pie = { enabled = true, x = 1495, y = 645, size = 4, colorkey = true }, -- Turning off colorkeying also maintains the original pie chart's dimensions and shows the percentages
	tall_pie = { enabled = true, x = 1495, y = 645, size = 4, colorkey = true }, -- Leave same as thin for seamlessness

	thin_percent = { enabled = false, x = 1568, y = 1050, size = 6 },
	tall_percent = { enabled = false, x = 1568, y = 1050, size = 6 }, -- Leave same as thin for seamlessness
	percentages_match_text = false, -- Enabling this makes the percentages match the text color rather than the pie colors

	measuring_window = { x = 30, y = 340, size = 14 },
	stretched_measure = false,

	-- ==== MACROS ====
	-- resolution changes
	tall = { key = "*-G", f3_safe = false, ingame_only = false },
	thin = { key = "*-B", f3_safe = false, ingame_only = true },
	wide = { key = "*-Z", f3_safe = true, ingame_only = true },

	-- during game actions
	toggle_remaps_key = "Delete",

	-- ==== NINJABRAIN BOT ====
	nbb = {
		keys = {
			decrement = "*-bracketleft", -- [
			increment = "*-bracketright", -- ]
			undo = "*-semicolon", -- ;
			redo = "*-apostrophe", -- '
			reset = "*-backslash", -- \
		},
	},

	-- ==== KEYBOARD ====
	xkb_config = { -- set any setting to nil if unwanted
		enabled = false,
		layout = "mc", -- ~/.config/xkb/symbols/mc
		rules = nil, -- ~/.config/xkb/rules/...
		variant = "basic",
		options = "caps:none",
	},
	remaps_text_config = { text = "chat mode", x = 100, y = 100, size = 2, color = "#000000" },

	-- ==== MISC ====
	-- see https://github.com/Esensats/mcsr-calcsens to configure sensitivity, e.g.
	-- python calcsens.py --normalRes 2560 1440 --tallRes 384 16384 0.5 1.0
	sens_change = { enabled = true, normal = 12.800000599064097, tall = 1.151214098039685, raw_input = false },
	enable_resize_animations = false,
}

local remaps = {
	remapped_kb = {
		-- Add any playing remaps here
		["LEFTALT"] = "F3",
	},

	normal_kb = {
		-- Add any remaps you want to keep when disabling normal remaps (not necessary)
	},
}

return main(cfg, remaps)
