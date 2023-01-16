require("mason").setup()

require("mason-lspconfig").setup({
	ensure_installed = {
		"bashls",
		"clangd",
		"pyright",
		"rust_analyzer",
		"sumneko_lua",
	},
	automatic_installation = true,
})
