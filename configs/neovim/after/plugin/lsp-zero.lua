local lsp = require("lsp-zero")
lsp.preset("recommended")

-- Configure lua language server for neovim
lsp.nvim_workspace()
lsp.setup()

require("lspconfig").clangd.setup{}
