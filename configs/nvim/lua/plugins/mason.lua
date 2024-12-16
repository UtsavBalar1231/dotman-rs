return {
	"williamboman/mason.nvim",

	dependencies = {
		"williamboman/mason-lspconfig.nvim",
		"WhoIsSethDaniel/mason-tool-installer.nvim",
		"jay-babu/mason-null-ls.nvim",
	},

	config = function()
		-- import mason
		local mason = require("mason")
		local mason_lspconfig = require("mason-lspconfig")
		local mason_tool_installer = require("mason-tool-installer")

		-- enable mason and configure icons
		mason.setup({
			ui = {
				icons = {
					package_installed = "",
					package_pending = "",
					package_uninstalled = "",
				},
			},
		})

		mason_lspconfig.setup({
			-- list of servers for mason to install
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

		mason_tool_installer.setup({
			ensure_installed = {
				"black",
				"clang-format",
				"clangd",
				"efm",
				"esbonio",
				"eslint",
				"eslint_d",
				"html",
				"marksman",
				"mdformat",
				"prettier",
				"prettierd",
				"pylint",
				"pyright",
				"rust_analyzer",
				"stylelint",
				"stylua",
				"yamlfmt",
				"yamllint",
			},
		})
	end,
}
