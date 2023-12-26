local status_ok, lspconfig = pcall(require, "lspconfig")

if not status_ok then
	vim.notify("Missing lspconfig plugin", vim.log.levels.WARNING)
	return
end

local status_ok_lspstatus, lsp_status = pcall(require, "lsp-status")
if not status_ok_lspstatus then
	vim.notify("Missing lsp-status plugin", vim.log.levels.WARN)
	return
end

lsp_status.register_progress()

-- Enable diagnostics
local signs = { Error = " ", Warn = " ", Hint = " ", Info = " " }
for type, icon in pairs(signs) do
	local hl = "DiagnosticSign" .. type
	vim.fn.sign_define(hl, { text = icon, texthl = hl })
end

vim.lsp.handlers["textDocument/publishDiagnostics"] = vim.lsp.with(vim.lsp.diagnostic.on_publish_diagnostics, {
	virtual_text = true,
	signs = true,
	update_in_insert = false,
})

vim.api.nvim_create_autocmd("LspAttach", {
	group = vim.api.nvim_create_augroup("UserLspConfig", {}),
	callback = function(ev)
		-- Enable completion triggered by <c-x><c-o>
		vim.bo[ev.buf].omnifunc = "v:lua.vim.lsp.omnifunc"

		-- Buffer local mappings.
		-- See `:help vim.lsp.*` for documentation on any of the below functions
		local opts = { buffer = ev.buf }
		vim.keymap.set("n", "gD", vim.lsp.buf.declaration, opts)
		vim.keymap.set("n", "gd", vim.lsp.buf.definition, opts)
		vim.keymap.set("n", "K", vim.lsp.buf.hover, opts)
		vim.keymap.set("n", "gi", vim.lsp.buf.implementation, opts)
		vim.keymap.set("n", "<C-k>", vim.lsp.buf.signature_help, opts)
		vim.keymap.set("n", "wa", vim.lsp.buf.add_workspace_folder, opts)
		vim.keymap.set("n", "wr", vim.lsp.buf.remove_workspace_folder, opts)
		vim.keymap.set("n", "wl", function()
			print(vim.inspect(vim.lsp.buf.list_workleader_folders()))
		end, opts)
		vim.keymap.set("n", "D", vim.lsp.buf.type_definition, opts)
		vim.keymap.set("n", "rn", vim.lsp.buf.rename, opts)
		vim.keymap.set({ "n", "v" }, "ca", vim.lsp.buf.code_action, opts)
		vim.keymap.set("n", "gr", vim.lsp.buf.references, opts)
		vim.keymap.set("n", "F", function()
			vim.lsp.buf.format({ async = true })
		end, opts)
	end,
})

-- asm/nasm
lspconfig.asm_lsp.setup({
	capabilities = lsp_status.capabilities,
	on_attach = lsp_status.on_attach,
})

-- bash / shell
lspconfig.bashls.setup({
	capabilities = lsp_status.capabilities,
	on_attach = lsp_status.on_attach,
})

-- c/c++
lspconfig.clangd.setup({
	capabilities = lsp_status.capabilities,
	on_attach = lsp_status.on_attach,
})

-- css
lspconfig.cssls.setup({
	capabilities = lsp_status.capabilities,
	on_attach = lsp_status.on_attach,
})

-- efm
local shellcheck = {
	lintCommand = "shellcheck",
	lintStdin = true,
}

local shfmt = {
	formatCommand = "shfmt",
	formatStdin = true,
}

local prettier = {
	formatCommand = "prettier",
	formatStdin = true,
}

local yamlfmt = {
	formatCommand = "yamlfmt",
	formatStdin = true,
}

local rst_pandoc = {
	formatCommand = "pandoc -f rst -t rst -s --columns=79",
	formatStdin = true,
}

lspconfig.efm.setup({
	on_attach = lsp_status.on_attach,
	capabilities = lsp_status.capabilities,
	root_dir = require("lspconfig/util").root_pattern(
		".clang-format",
		".eslintrc",
		".eslintrc.json",
		".luacheck",
		".markdownlint.json",
		".markdownlint.yaml",
		".prettierrc",
		".prettierrc.json",
		".vale.ini",
		"cargo.toml",
		"dpring.json",
		"go.mod",
		"package.json",
		"stylua.toml"
	),
	init_options = {
		codeAction = true,
		completion = true,
		documentFormatting = true,
		documentRangeFormatting = true,
		documentSymbol = true,
		hover = true,
	},
	settings = {
		rootMarkers = {
			".git/",
			".zshrc",
			"cargo.toml",
			"go.mod",
			"package.json",
			"vale.ini",
		},
		languages = {
			bash = { shellcheck, shfmt },
			sh = { shellcheck, shfmt },
			css = { prettier },
			yaml = { prettier, yamlfmt },
			rst = { rst_pandoc },
			restructuredtext = { rst_pandoc },
		},
	},
})

-- esbonio (sphinx)
lspconfig.esbonio.setup({
	capabilities = lsp_status.capabilities,
	on_attach = lsp_status.on_attach,
})

-- go lang
lspconfig.gopls.setup({
	capabilities = lsp_status.capabilities,
	on_attach = lsp_status.on_attach,
})

-- html
lspconfig.html.setup({
	capabilities = lsp_status.capabilities,
	on_attach = lsp_status.on_attach,
})

-- json
lspconfig.jsonls.setup({
	capabilities = lsp_status.capabilities,
	on_attach = lsp_status.on_attach,
})

-- lua
lspconfig.lua_ls.setup({
	capabilities = lsp_status.capabilities,
	on_attach = lsp_status.on_attach,
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
	capabilities = lsp_status.capabilities,
	on_attach = lsp_status.on_attach,
})

-- python
lspconfig.pyright.setup({
	capabilities = lsp_status.capabilities,
	on_attach = lsp_status.on_attach,
})

-- rust
lspconfig.rust_analyzer.setup({
	capabilities = lsp_status.capabilities,
	-- on_attach is set from rust.lua
})

-- SystemVerilog
lspconfig.svls.setup({
	capabilities = lsp_status.capabilities,
	on_attach = lsp_status.on_attach,
})

-- typescript
lspconfig.tsserver.setup({
	capabilities = lsp_status.capabilities,
	on_attach = lsp_status.on_attach,
})

-- vim
lspconfig.vimls.setup({
	capabilities = lsp_status.capabilities,
	on_attach = lsp_status.on_attach,
})

lspconfig.lemminx.setup({
	capabilities = lsp_status.capabilities,
	on_attach = lsp_status.on_attach,
})

-- yaml
-- lspconfig.yamlls.setup({
-- 	capabilities = lsp_status.capabilities,
-- 	on_attach = lsp_status.on_attach,
-- })
