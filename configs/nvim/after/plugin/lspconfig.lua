local status_ok, lspconfig = pcall(require, "lspconfig")

if not status_ok then
	return
end

lspconfig.bashls.setup({
	on_attach = function(client, _)
		client.server_capabilities.documentFormattingProvider = false
	end,

	root_dir = function()
		return vim.loop.cwd()
	end,
})

lspconfig.lua_ls.setup({
	settings = {
		Lua = {
			diagnostics = {
				globals = "vim",
			},
		},
	},
	on_attach = function(client, _)
		client.server_capabilities.documentFormattingProvider = false
	end,
})

lspconfig.rust_analyzer.setup({
	on_attach = function(client, _)
		client.server_capabilities.documentFormattingProvider = false
	end,
	root_dir = function()
		return vim.loop.cwd()
	end,
})

lspconfig.zls.setup({
	on_attach = function(client, _)
		client.server_capabilities.documentFormattingProvider = false
	end,
	root_dir = function()
		return vim.loop.cwd()
	end,
})
