local status_ok, lspconfig = pcall(require, "lspconfig")

if not status_ok then
	vim.notify("Missing lspconfig dependency", vim.log.levels.ERROR)
	return
end

local status_ok_lspstatus, lsp_status = pcall(require, "lsp-status")
if not status_ok_lspstatus then
	vim.notify("lsp-status not found", vim.log.levels.WARN)
	return
end

lsp_status.register_progress()

-- Enable diagnostics
vim.lsp.handlers["textDocument/publishDiagnostics"] = vim.lsp.with(vim.lsp.diagnostic.on_publish_diagnostics, {
	virtual_text = true,
	signs = true,
	update_in_insert = false,
})
vim.fn.sign_define("LspDiagnosticsSignError", { text = "", texthl = "LspDiagnosticsSignError" })
vim.fn.sign_define("LspDiagnosticsSignWarning", { text = "", texthl = "LspDiagnosticsSignWarning" })
vim.fn.sign_define("LspDiagnosticsSignInformation", { text = "", texthl = "LspDiagnosticsSignInformation" })
vim.fn.sign_define("LspDiagnosticsSignHint", { text = "", texthl = "LspDiagnosticsSignHint" })

local languages = require("efmls-configs.defaults").languages()
local efmls_config = {
	filetypes = vim.tbl_keys(languages),
	settings = {
		rootMarkers = { ".git/" },
		languages = languages,
	},
	init_options = {
		codeAction = true,
		completion = true,
		documentFormatting = true,
		documentRangeFormatting = true,
		documentSymbol = true,
		hover = true,
	},
}

require("lspconfig").efm.setup(vim.tbl_extend("force", efmls_config, {
	on_attach = lsp_status.on_attach,
	capabilities = lsp_status.capabilities,
}))

local black = {
	lintCommand = "black",
	lintStdin = true,
	formatCommand = "black",
	formatStdin = true,
}
local clang_format = require("efmls-configs.formatters.clang_format")
local clang_tidy = require("efmls-configs.linters.clang_tidy")
local cpplint = require("efmls-configs.linters.cpplint")
local dprint = require("efmls-configs.formatters.dprint")
local eslint = require("efmls-configs.linters.eslint")
local flake8 = require("efmls-configs.linters.flake8")
local gofmt = require("efmls-configs.formatters.gofmt")
local goimports = require("efmls-configs.formatters.goimports")
local golines = require("efmls-configs.formatters.golines")
local golint = require("efmls-configs.linters.golint")
local jq = {
	lintCommand = "jq .",
	lintStdin = true,
	formatCommand = "jq .",
	formatStdin = true,
}
local languagetool = require("efmls-configs.linters.languagetool")
local luacheck = require("efmls-configs.linters.luacheck")
local prettier = require("efmls-configs.formatters.prettier")
local rustfmt = require("efmls-configs.formatters.rustfmt")
local shellcheck = require("efmls-configs.linters.shellcheck")
local shfmt = require("efmls-configs.formatters.shfmt")
local stylelint = require("efmls-configs.linters.stylelint")
local stylua = require("efmls-configs.formatters.stylua")
local vale = require("efmls-configs.linters.vale")
local yamllint = require("efmls-configs.linters.yamllint")

-- efm
lspconfig.efm.setup({
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
			c = { clang_format, clang_tidy, cpplint },
			cpp = { clang_format, clang_tidy, cpplint },
			css = { prettier, stylelint },
			go = { gofmt, goimports, golines, golint },
			html = { prettier },
			javascript = { prettier },
			javascriptreact = { prettier },
			json = { prettier, jq },
			latex = { languagetool },
			lua = { stylua, luacheck },
			markdown = { dprint, vale },
			org = { vale },
			python = { black, flake8 },
			rust = { rustfmt },
			sh = { shellcheck, shfmt },
			toml = { dprint },
			txt = { vale },
			typescript = { prettier, eslint },
			typescriptreact = { prettier, eslint },
			yaml = { prettier, yamllint },
		},
	},
})

-- sh
-- bash
lspconfig.bashls.setup({
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

-- rust
lspconfig.rust_analyzer.setup({
	capabilities = lsp_status.capabilities,
	-- on_attach is set from rust.lua
})

-- css
lspconfig.cssls.setup({
	capabilities = lsp_status.capabilities,
	on_attach = lsp_status.on_attach,
})

-- c/c++
lspconfig.clangd.setup({
	capabilities = lsp_status.capabilities,
	on_attach = lsp_status.on_attach,
})

lspconfig.ccls.setup({
	capabilities = lsp_status.capabilities,
	on_attach = lsp_status.on_attach,
})

-- yaml
lspconfig.yamlls.setup({
	capabilities = lsp_status.capabilities,
	on_attach = lsp_status.on_attach,
})

-- json
lspconfig.jsonls.setup({
	capabilities = lsp_status.capabilities,
	on_attach = lsp_status.on_attach,
})

-- html
lspconfig.html.setup({
	capabilities = lsp_status.capabilities,
	on_attach = lsp_status.on_attach,
})

-- vim
lspconfig.vimls.setup({
	capabilities = lsp_status.capabilities,
	on_attach = lsp_status.on_attach,
})

-- python
lspconfig.pyright.setup({
	capabilities = lsp_status.capabilities,
	on_attach = lsp_status.on_attach,
})

-- marksman
lspconfig.marksman.setup({
	capabilities = lsp_status.capabilities,
	on_attach = lsp_status.on_attach,
})

-- SystemVerilog
lspconfig.svls.setup({
	capabilities = lsp_status.capabilities,
	on_attach = lsp_status.on_attach,
})
