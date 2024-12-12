return {
	"nvimtools/none-ls.nvim",
	dependencies = {
		"williamboman/mason.nvim",
		"williamboman/mason-lspconfig.nvim",
		"jay-babu/mason-null-ls.nvim",
	},

	config = function()
		local mason_null_ls = require("mason-null-ls")

		mason_null_ls.setup({
			ensure_installed = {
				"asm_lsp",
				"bashls",
				"black",
				"clangd",
				"cssls",
				"efm",
				"esbonio",
				"eslint",
				"eslint_d",
				"html",
				"jsonls",
				"lua_ls",
				"marksman",
				"prettier",
				"pylint",
				"pyright",
				"rust_analyzer",
				"rustfmt",
				"stylua",
				"svls",
				"vimls",
				"yamllint",
				"yamlls",
			},
		})

		local null_ls = require("null-ls")

		null_ls.setup({
			sources = {
				-- Completions
				null_ls.builtins.completion.luasnip,
				-- null_ls.builtins.completion.spell,
				null_ls.builtins.completion.tags,

				-- Diagnostics
				-- null_ls.builtins.diagnostics.stylelint,
				-- null_ls.builtins.diagnostics.checkmake,
				-- null_ls.builtins.diagnostics.commitlint,
				-- null_ls.builtins.diagnostics.eslint_d,
				-- null_ls.builtins.diagnostics.eslint,
				-- null_ls.builtins.diagnostics.markdownlint,
				null_ls.builtins.diagnostics.pylint,
				-- null_ls.builtins.diagnostics.rstcheck,
				null_ls.builtins.diagnostics.todo_comments,
				-- null_ls.builtins.diagnostics.trail_space,
				null_ls.builtins.diagnostics.zsh,

				-- Formatting
				null_ls.builtins.formatting.black,
				null_ls.builtins.formatting.clang_format,
				null_ls.builtins.formatting.gofmt,
				null_ls.builtins.formatting.htmlbeautifier,
				null_ls.builtins.formatting.mdformat,
				null_ls.builtins.formatting.prettier,
				null_ls.builtins.formatting.prettierd,
				null_ls.builtins.formatting.shfmt,
				null_ls.builtins.formatting.stylelint,
				null_ls.builtins.formatting.stylua,
				null_ls.builtins.formatting.yamlfmt,

				-- Hover options
				null_ls.builtins.hover.dictionary,
				null_ls.builtins.hover.printenv,
			},
		})
	end,
}
