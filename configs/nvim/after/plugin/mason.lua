local status_ok, mason = pcall(require, "mason")

if not status_ok then
	vim.notify("Missing mason.nvim dependency", vim.log.levels.ERROR)
	return
end

mason.setup({
	ui = {
		icons = {
			package_installed = "",
			package_pending = "",
			package_uninstalled = "",
		},
	},
})

local status_ok_mlsp, mason_lspconfig = pcall(require, "mason-lspconfig")

if not status_ok_mlsp then
	vim.notify("Missing mason-lspconfig.nvim dependency", vim.log.levels.ERROR)
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
