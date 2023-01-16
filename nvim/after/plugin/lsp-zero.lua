local lsp = require("lsp-zero")
lsp.preset("recommended")
-- Install these servers
lsp.ensure_installed({
	"sumneko_lua",
	"pyright",
	"clangd",
	"rust_analyzer",
	"bashls",
	"jsonls",
})

-- Configure lua language server for neovim
lsp.nvim_workspace()
lsp.setup()
