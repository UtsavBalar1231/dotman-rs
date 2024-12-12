return {
	"neovim/nvim-lspconfig",
	event = { "BufReadPre", "BufNewFile" },
	dependencies = {
		"hrsh7th/cmp-nvim-lsp",
		"j-hui/fidget.nvim",
		{ "antosha417/nvim-lsp-file-operations", config = true },
	},
	config = function()
		local lspconfig = require("lspconfig")
		local cmp_nvim_lsp = require("cmp_nvim_lsp")

		local keymap = vim.keymap
		local opts = { noremap = true, silent = true }

		local on_attach = function(_, bufnr)
			opts.buffer = bufnr

			-- set keybinds
			opts.desc = "LSP: Show references"
			keymap.set("n", "gR", "<cmd>Telescope lsp_references<CR>", opts) -- show definition, references

			opts.desc = "LSP: Go to declaration"
			keymap.set("n", "gD", vim.lsp.buf.declaration, opts) -- go to declaration

			opts.desc = "LSP: Show definitions"
			keymap.set("n", "gd", "<cmd>Telescope lsp_definitions<CR>", opts) -- show lsp definitions

			opts.desc = "LSP: Show implementations"
			keymap.set("n", "gi", "<cmd>Telescope lsp_implementations<CR>", opts) -- show lsp implementations

			opts.desc = "LSP: Show type definitions"
			keymap.set("n", "gt", "<cmd>Telescope lsp_type_definitions<CR>", opts) -- show lsp type definitions

			opts.desc = "LSP: See available code actions"
			keymap.set({ "n", "v" }, "<leader>ca", vim.lsp.buf.code_action, opts) -- see available code actions, in visual mode will apply to selection

			opts.desc = "LSP: Smart rename"
			keymap.set("n", "<leader>rn", vim.lsp.buf.rename, opts) -- smart rename

			opts.desc = "LSP: Show buffer diagnostics"
			keymap.set("n", "<leader>D", "<cmd>Telescope diagnostics bufnr=0<CR>", opts) -- show  diagnostics for file

			opts.desc = "LSP: Show line diagnostics"
			keymap.set("n", "<leader>d", vim.diagnostic.open_float, opts) -- show diagnostics for line

			opts.desc = "LSP: Go to previous diagnostic"
			keymap.set("n", "[d", vim.diagnostic.goto_prev, opts) -- jump to previous diagnostic in buffer

			opts.desc = "LSP: Go to next diagnostic"
			keymap.set("n", "]d", vim.diagnostic.goto_next, opts) -- jump to next diagnostic in buffer

			opts.desc = "LSP: Show documentation for what is under cursor"
			keymap.set("n", "K", vim.lsp.buf.hover, opts) -- show documentation for what is under cursor

			opts.desc = "Restart LSP"
			keymap.set("n", "<leader>rs", ":LspRestart<CR>", opts) -- mapping to restart lsp if necessary

			opts.desc = "LSP Format buffer"
			keymap.set("n", "F", function()
				vim.lsp.buf.format({ async = true })
			end, opts)

			opts.desc = "LSP Signature help"
			keymap.set("n", "<C-k>", vim.lsp.buf.signature_help, opts)
		end

		-- used to enable autocompletion (assign to every lsp server config)
		local capabilities = cmp_nvim_lsp.default_capabilities()

		-- Change the Diagnostic symbols in the sign column (gutter)
		local signs = { Error = " ", Warn = " ", Hint = " ", Info = " " }
		for type, icon in pairs(signs) do
			local hl = "DiagnosticSign" .. type
			vim.fn.sign_define(hl, { text = icon, texthl = hl, numhl = "" })
		end

		-- configure lua server (with special settings)
		lspconfig.lua_ls.setup({
			capabilities = capabilities,
			on_attach = on_attach,
			settings = { -- custom settings for lua
				Lua = {
					-- make the language server recognize "vim" global
					diagnostics = {
						globals = { "vim" },
					},
					workspace = {
						-- make language server aware of runtime files
						library = {
							[vim.fn.expand("$VIMRUNTIME/lua")] = true,
							[vim.fn.stdpath("config") .. "/lua"] = true,
						},
					},
				},
			},
		})

		-- asm/nasm
		lspconfig.asm_lsp.setup({
			capabilities = capabilities,
			on_attach = on_attach,
		})

		-- bash / shell
		lspconfig.bashls.setup({
			capabilities = capabilities,
			on_attach = on_attach,
		})

		-- c/c++
		lspconfig.clangd.setup({
			capabilities = capabilities,
			on_attach = on_attach,
			cmd = {
				"clangd",
				"--offset-encoding=utf-16",
				-- this causes a dot symbol next to completion and Neovim does not handle
				-- this well.
				"--header-insertion=never",
			},
		})

		-- css
		lspconfig.cssls.setup({
			capabilities = capabilities,
			on_attach = on_attach,
		})

		-- esbonio (sphinx)
		lspconfig.esbonio.setup({
			capabilities = capabilities,
			on_attach = on_attach,
		})

		-- go lang
		-- lspconfig.gopls.setup({
		-- 	capabilities = capabilities,
		-- 	on_attach = on_attach,
		-- })

		-- html
		lspconfig.html.setup({
			capabilities = capabilities,
			on_attach = on_attach,
		})

		-- json
		lspconfig.jsonls.setup({
			capabilities = capabilities,
			on_attach = on_attach,
		})

		lspconfig.marksman.setup({
			capabilities = capabilities,
			on_attach = on_attach,
		})

		-- python
		lspconfig.pyright.setup({
			capabilities = capabilities,
			on_attach = on_attach,
		})

		-- rust
		lspconfig.rust_analyzer.setup({
			capabilities = capabilities,
			-- on_attach is set from rust.lua
		})

		-- SystemVerilog
		lspconfig.svls.setup({
			capabilities = capabilities,
			on_attach = on_attach,
		})

		-- typescript
		lspconfig.ts_ls.setup({
			capabilities = capabilities,
			on_attach = on_attach,
		})

		-- vim
		lspconfig.vimls.setup({
			capabilities = capabilities,
			on_attach = on_attach,
		})

		-- xml
		lspconfig.lemminx.setup({
			capabilities = capabilities,
			on_attach = on_attach,
		})

		-- yaml
		lspconfig.yamlls.setup({
			capabilities = capabilities,
			on_attach = on_attach,
		})

		-- c3 lsp
		local lsp_configurations = require("lspconfig.configs")

		if not lsp_configurations.c3_lsp then
			lsp_configurations.c3_lsp = {
				default_config = {
					name = "c3_lsp",
					cmd = {
						"/usr/local/bin/c3-lsp",
					},
					filetypes = { "c3" },
					root_dir = require("lspconfig.util").root_pattern(".git", "CMakeLists.txt"),
				},
			}
		end

		lspconfig.c3_lsp.setup({
			capabilities = capabilities,
			on_attach = on_attach,
		})

		vim.diagnostic.config({
			-- update_in_insert = true,
			float = {
				focusable = false,
				style = "minimal",
				border = "rounded",
				header = "",
				prefix = "",
			},
		})
	end,
}
