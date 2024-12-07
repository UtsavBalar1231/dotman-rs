local status_ok, lspconfig = pcall(require, "lspconfig")

if not status_ok then
	vim.notify("Missing lspconfig plugin", vim.log.levels.WARNING)
	return
end

local config = {
	virtual_text = false, -- use trouble instead, or [d, ]d keymap
	signs = {
		text = {
			[vim.diagnostic.severity.ERROR] = "",
			[vim.diagnostic.severity.WARN] = "",
			[vim.diagnostic.severity.HINT] = "",
			[vim.diagnostic.severity.INFO] = "",
		},
		texthl = {
			[vim.diagnostic.severity.ERROR] = "DiagnosticSignError",
			[vim.diagnostic.severity.WARN] = "DiagnosticSignWarn",
			[vim.diagnostic.severity.HINT] = "DiagnosticSignHint",
			[vim.diagnostic.severity.INFO] = "DiagnosticSignInfo",
		},
		numhl = {
			[vim.diagnostic.severity.ERROR] = "",
			[vim.diagnostic.severity.WARN] = "",
			[vim.diagnostic.severity.HINT] = "",
			[vim.diagnostic.severity.INFO] = "",
		},
	},
	update_in_insert = true,
	underline = true,
	severity_sort = true,
	float = {
		focusable = true,
		style = "minimal",
		border = nil,
		source = "always",
		header = "",
		prefix = "",
	},
}
vim.diagnostic.config(config)

vim.lsp.handlers["textDocument/publishDiagnostics"] = vim.lsp.with(vim.lsp.diagnostic.on_publish_diagnostics, {
	virtual_text = true,
	signs = true,
	update_in_insert = false,
})

--- Enable floating window for diagnostics
--- Map to <C-e> to toggle
vim.api.nvim_set_keymap(
	"n",
	"<leader>e",
	"<cmd>lua vim.diagnostic.open_float(0, { focus=true })<CR>",
	{ noremap = true, silent = true }
)

-- Map <leader>d[ to goto previous diagnostic
vim.keymap.set("n", "<leader>d[", function()
	vim.diagnostic.goto_prev({ popup_opts = { border = "rounded" } })
end, {})

-- Map <leader>f] to goto next diagnostic
vim.keymap.set("n", "<leader>d]", function()
	vim.diagnostic.goto_next({ popup_opts = { border = "rounded" } })
end, {})

local capabilities = require("cmp_nvim_lsp").default_capabilities(vim.lsp.protocol.make_client_capabilities())

local on_attach = function(_, bufnr)
	local function buf_set_option(...)
		vim.api.nvim_buf_set_option(bufnr, ...)
	end

	buf_set_option("omnifunc", "v:lua.vim.lsp.omnifunc")
	local bufopts = { noremap = true, silent = true, buffer = bufnr }

	vim.keymap.set("n", "gD", vim.lsp.buf.declaration, bufopts)
	vim.keymap.set("n", "gd", vim.lsp.buf.definition, bufopts)
	vim.keymap.set("n", "K", vim.lsp.buf.hover, bufopts)
	vim.keymap.set("n", "gi", vim.lsp.buf.implementation, bufopts)
	vim.keymap.set("n", "<C-k>", vim.lsp.buf.signature_help, bufopts)
	vim.keymap.set("n", "<leader>wa", vim.lsp.buf.add_workspace_folder, bufopts)
	vim.keymap.set("n", "<leader>wr", vim.lsp.buf.remove_workspace_folder, bufopts)
	vim.keymap.set("n", "<leader>wl", function()
		print(vim.inspect(vim.lsp.buf.list_workspace_folders()))
	end, bufopts)
	vim.keymap.set("n", "<leader>D", vim.lsp.buf.type_definition, bufopts)
	vim.keymap.set("n", "<leader>rn", vim.lsp.buf.rename, bufopts)
	vim.keymap.set("n", "<leader>fa", vim.lsp.buf.code_action, bufopts)
	vim.keymap.set("n", "gr", vim.lsp.buf.references, bufopts)

	vim.keymap.set("n", "F", function()
		vim.lsp.buf.format({ async = true })
	end, bufopts)
end

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

-- lua
lspconfig.lua_ls.setup({
	capabilities = capabilities,
	on_attach = on_attach,
	settings = {
		Lua = {
			runtime = { version = "LuaJIT" },
			diagnostics = { globals = { "vim" } },
			workspace = {
				library = vim.api.nvim_get_runtime_file("", true),
				checkThirdParty = false,
			},
			telemetry = { enable = false },
		},
	},
})

-- marksman
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
