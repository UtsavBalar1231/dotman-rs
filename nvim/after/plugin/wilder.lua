local wilder = require("wilder")
wilder.setup({
	modes = { ":", "/", "?" },
})

wilder.set_option("use_python_remote_plugin", 0)

wilder.set_option("pipeline", {
	wilder.branch(
		wilder.cmdline_pipeline({
			use_python = 0,
			fuzzy = 1,
			fuzzy_filter = wilder.lua_fzy_filter(),
		}),
		wilder.vim_search_pipeline({
			use_python = 0,
			fuzzy = 1,
			fuzzy_filter = wilder.lua_fzy_filter(),
		}),
		wilder.history()
	),
})
local gruvbox_fg_color = "#79740e"
local gruvbox_bg_color = "#fbf1c7"

local create_highlight = function(type)
	local ret = {
		border = { "rounded" },
		prompt = { "Normal" },
		accent = wilder.make_hl("WilderAccen" .. type, "PMenu", {
			{ a = 1 },
			{ a = 1 },
			{ background = gruvbox_bg_color, foreground = gruvbox_fg_color, bold = true },
		}),
	}

	return ret
end

wilder.set_option(
	"renderer",
	wilder.renderer_mux({
		[":"] = wilder.popupmenu_renderer({
			highlighter = wilder.lua_fzy_highlighter(),
			highlights = create_highlight("Popup"),
		}),
		["/"] = wilder.wildmenu_renderer({
			highlighter = wilder.lua_fzy_highlighter(),
			highlights = create_highlight("Mini"),
		}),
		["?"] = wilder.wildmenu_renderer({
			highlighter = wilder.lua_fzy_highlighter(),
			highlights = create_highlight("Mini"),
		}),
	})
)
