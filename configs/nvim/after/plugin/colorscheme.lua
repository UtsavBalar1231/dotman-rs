-- Setup gruvbox_material theme
vim.g.gruvbox_material_background = "hard"
vim.g.gruvbox_material_enable_italic = 1
vim.g.gruvbox_material_enable_bold = 1
vim.g.gruvbox_material_palette = "mix"
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

-- Set background to dark
vim.opt.background = "dark"

-- Enable filetype plugin support
vim.opt.filetype:append("plugin")

vim.opt.syntax = "on"
