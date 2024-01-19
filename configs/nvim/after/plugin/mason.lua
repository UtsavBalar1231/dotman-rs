local status_ok, mason = pcall(require, "mason")

if not status_ok then
	vim.notify("Missing mason.nvim plugin", vim.log.levels.WARNING)
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
	vim.notify("Missing mason-lspconfig.nvim plugin", vim.log.levels.WARNING)
	return
end

mason_lspconfig.setup({
	ensure_installed = {
		"asm_lsp",
		"bashls",
		"clangd",
		"cssls",
		"efm",
		"esbonio",
		"html",
		"jsonls",
		"lua_ls",
		"marksman",
		"pyright",
		"rust_analyzer",
		"svls",
		"vimls",
		"yamlls",
	},
	automatic_installation = true,
})
