-- Setup gruvbox theme
require("gruvbox").setup({ palette_overrides = { dark0_hard = "#282828" } })
vim.cmd("colorscheme gruvbox")

-- Set background to dark
vim.opt.background = "dark"

-- Enable filetype plugin indent support
vim.opt.filetype:append("plugin")
vim.opt.filetype:append("indent")

vim.opt.syntax = "on"
