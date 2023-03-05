local status_ok, mason = pcall(require, "mason")
if not status_ok then
	return
end

mason.setup()

local status_ok_mlsp, mason_lspconfig = pcall(require, "mason-lspconfig")

if not status_ok_mlsp then
	return
end

mason_lspconfig.setup({
	ensure_installed = {
		"bashls",
		"clangd",
		"pyright",
		"rust_analyzer",
		"lua_ls",
		"vimls",
	},
	automatic_installation = true,
})
