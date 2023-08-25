-- Setup gruvbox_material theme
-- vim.g.gruvbox_material_background = "hard"
-- vim.g.gruvbox_material_enable_italic = 1
-- vim.g.gruvbox_material_enable_bold = 1
-- vim.g.gruvbox_material_palette = "mix"
-- vim.g.gruvbox_material_better_performance = 1
-- vim.g.gruvbox_material_diagnostic_text_highlight = 1
-- vim.g.gruvbox_material_diagnostic_line_highlight = 1
-- vim.g.gruvbox_material_diagnostic_virtual_text = "colored"
-- vim.g.gruvbox_material_diagnostic_underline = 1
-- vim.g.gruvbox_material_diagnostic_signs = 1
-- vim.g.gruvbox_material_current_word = "bold"
-- vim.g.gruvbox_material_cursor = "red"
-- vim.g.gruvbox_material_dim_inactive_windows = 1
-- vim.cmd([[colorscheme gruvbox-material]])

require("catppuccin").setup({
	flavour = "frappe", -- latte, frappe, macchiato, mocha
	background = {  -- :h background
		light = "latte",
		dark = "frappe",
	},
	transparent_background = false, -- disables setting the background color.
	show_end_of_buffer = false,  -- shows the '~' characters after the end of buffers
	term_colors = false,         -- sets terminal colors (e.g. `g:terminal_color_0`)
	dim_inactive = {
		enabled = true,
		shade = "dark",
		percentage = 0.15,
	},
	no_italic = false,     -- Force no italic
	no_bold = false,       -- Force no bold
	no_underline = false,  -- Force no underline
	styles = {             -- Handles the styles of general hi groups (see `:h highlight-args`):
		comments = { "italic" }, -- Change the style of comments
		conditionals = { "italic" },
		loops = {},
		functions = {},
		keywords = {},
		strings = {},
		variables = {},
		numbers = {},
		booleans = {},
		properties = {},
		types = {},
		operators = {},
	},
	color_overrides = {
		frappe = {
			base = "#141414",
		},
		mocha = {
			base = "#111111",
		},
	},
	custom_highlights = {},
	integrations = {
		cmp = true,
		gitsigns = true,
		hop = true,
		indent_blankline = {
			colored_indent_levels = true,
			enabled = true,
		},
		mason = true,
		mini = false,
		notify = false,
		nvimtree = true,
		telescope = {
			enabled = true,
		},
		treesitter = true,
		treesitter_context = false,
	},
})

vim.cmd.colorscheme("catppuccin")

-- Set background to dark
vim.opt.background = "dark"

-- Enable filetype plugin support
vim.opt.filetype:append("plugin")

vim.opt.syntax = "on"
