local status_ok, lspconfig = pcall(require, "lspconfig")

if not status_ok then
	return
end

lspconfig.bashls.setup({
	on_attach = function(client, _)
		client.server_capabilities.documentFormattingProvider = false
	end,
})

lspconfig.lua_ls.setup({
	settings = {
		Lua = {
			runtime = {
				version = "LuaJIT",
			},
			diagnostics = {
				globals = { "vim" },
			},
			workspace = {
				library = vim.api.nvim_get_runtime_file("", true),
			},
			telemetry = {
				enable = false,
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
})

lspconfig.zls.setup({
	on_attach = function(client, _)
		client.server_capabilities.documentFormattingProvider = false
	end,
})
