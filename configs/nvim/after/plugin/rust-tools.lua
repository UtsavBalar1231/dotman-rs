local status_ok, treesitter_configs = pcall(require, "rust-tools")

if not status_ok then
	vim.notify("Missing rust-tools plugin", vim.log.levels.WARNING)
	return
end

local function on_attach(client, buffer)
	if client.server_capabilities.documentFormattingProvider then
		local au_lsp = vim.api.nvim_create_augroup("rust_lsp", { clear = true })
		vim.api.nvim_create_autocmd("BufWritePre", {
			pattern = "*",
			callback = function()
				vim.lsp.buf.format({ async = false })
			end,
			group = au_lsp,
		})
	end
	-- Show diagnostic popup on cursor hover
	local diag_float_grp = vim.api.nvim_create_augroup("DiagnosticFloat", { clear = true })
	vim.api.nvim_create_autocmd("CursorHold", {
		callback = function()
			vim.diagnostic.open_float(nil, { focusable = false })
		end,
		group = diag_float_grp,
	})
end

-- Configure LSP through rust-tools.nvim plugin.
-- rust-tools will configure and enable certain LSP features for us.
-- See https://github.com/simrat39/rust-tools.nvim#configuration
local opts = {
	tools = {
		autoSetHints = true,
		runnables = {
			use_telescope = true,
		},
		debuggables = {
			use_telescope = true,
		},
		inlay_hints = {
			auto = true,
			show_parameter_hints = false,
			parameter_hints_prefix = "",
			other_hints_prefix = "",
		},
		hover_actions = {
			border = {
				{ "╭", "FloatBorder" },
				{ "─", "FloatBorder" },
				{ "╮", "FloatBorder" },
				{ "│", "FloatBorder" },
				{ "╯", "FloatBorder" },
				{ "─", "FloatBorder" },
				{ "╰", "FloatBorder" },
				{ "│", "FloatBorder" },
			},
			auto_focus = true,
		},
	},

	server = {
		cmd = { "rustup", "run", "nightly", "rust-analyzer" },

		-- on_attach is a callback called when the language server attachs to the buffer
		on_attach = on_attach,
		settings = {
			["rust-analyzer"] = {
				assist = {
					importEnforceGranularity = true,
					importPrefix = "crate",
				},
				inlayHints = {
					lifetimeElisionHints = {
						enable = true,
						useParameterNames = true,
					},
				},
				cargo = {
					allFeatures = true,
				},
				checkOnSave = {
					enable = true,
					command = "clippy",
				},
				procMacro = {
					enable = true,
				},
			},
		},
	},
}

require("rust-tools").setup(opts)
