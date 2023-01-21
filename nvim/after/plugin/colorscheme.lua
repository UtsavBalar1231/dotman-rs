-- Setup gruvbox_material theme
vim.g.gruvbox_material_background = "hard"
vim.g.gruvbox_material_enable_italic = 1
vim.g.gruvbox_material_enable_bold = 1
vim.g.gruvbox_material_palette = "material"
vim.g.gruvbox_material_better_performance = 1
vim.g.gruvbox_material_diagnostic_text_highlight = 1
vim.g.gruvbox_material_diagnostic_line_highlight = 1
vim.g.gruvbox_material_diagnostic_virtual_text = "colored"
vim.g.gruvbox_material_diagnostic_underline = 1
vim.g.gruvbox_material_diagnostic_signs = 1
vim.g.gruvbox_material_current_word = "bold"
vim.g.gruvbox_material_cursor = "red"
vim.g.gruvbox_material_dim_inactive_windows = 1
vim.cmd([[colorscheme gruvbox-material]])

-- gruvbox colors
local colors = {
	black = "#282828",
	red = "#cc241d",
	green = "#98971a",
	yellow = "#d79921",
	blue = "#458588",
	purple = "#b16286",
	cyan = "#689d6a",
	white = "#a89984",
	fg = "#ebdbb2",
	bg = "#282828",
}

vim.api.nvim_set_hl(0, 'Pmenu', { bg = colors.bg1, fg = colors.fg })

-- Set background to dark
vim.opt.background = "dark"

-- Enable filetype plugin indent support
vim.opt.filetype:append("plugin")
vim.opt.filetype:append("indent")

vim.opt.syntax = "on"
