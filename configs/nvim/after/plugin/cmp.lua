local status_ok, cmp = pcall(require, "cmp")

if not status_ok then
	return
end

local status_ok_lspkind, lspkind = pcall(require, "lspkind")

if not status_ok_lspkind then
	return
end

cmp.setup({
	formatting = {
		format = function(entry, vim_item)
			vim_item.kind = lspkind.presets.default[vim_item.kind]
			vim_item.menu = ({
				nvim_lsp = "[LSP]",
				nvim_lua = "[Lua]",
				vsnip = "[VSnip]",
				path = "[Path]",
				buffer = "[Buffer]",
				cmdline = "[Cmd]",
			})[entry.source.name]
			return vim_item
		end,
	},
	preselect = cmp.PreselectMode.None,

	snippet = {
		-- use vsnip
		expand = function(args)
			vim.fn["vsnip#anonymous"](args.body)
		end,
	},

	mapping = {
		["<C-Down>"] = cmp.mapping(cmp.mapping.select_next_item(), { "i", "c" }),
		["<C-Up>"] = cmp.mapping(cmp.mapping.select_prev_item(), { "i", "c" }),
		-- Add tab support
		["<S-Tab>"] = cmp.mapping.select_prev_item(),
		["<Tab>"] = cmp.mapping.select_next_item(),
		["<C-n>"] = cmp.mapping(cmp.mapping.scroll_docs(-4), { "i", "c" }),
		["<C-p>"] = cmp.mapping(cmp.mapping.scroll_docs(4), { "i", "c" }),
		["<C-Space>"] = cmp.mapping(cmp.mapping.complete(), { "i", "c" }),
		["<C-e>"] = cmp.mapping({ i = cmp.mapping.close(), c = cmp.mapping.close() }),
		["<CR>"] = cmp.mapping({
			i = cmp.mapping.confirm({ behavior = cmp.ConfirmBehavior.Replace, select = false }),
			c = function(fallback)
				if cmp.visible() then
					cmp.confirm({ behavior = cmp.ConfirmBehavior.Replace, select = false })
				else
					fallback()
				end
			end,
		}),
	},

	-- Installed sources
	sources = {
		{ name = "nvim_lsp" },
		{ name = "nvim_lua" },
		{ name = "vsnip" },
		{ name = "path" },
		{ name = "buffer" },
		{ name = "cmdline" },
	},
})

-- LSP Diagnostics Options Setup
local sign = function(sign_opts)
	vim.fn.sign_define(sign_opts.name, {
		texthl = sign_opts.name,
		text = sign_opts.text,
		numhl = "",
	})
end

sign({ name = "DiagnosticSignError", text = "" })
sign({ name = "DiagnosticSignWarn", text = "" })
sign({ name = "DiagnosticSignHint", text = "" })
sign({ name = "DiagnosticSignInfo", text = "" })

-- vim.diagnostic.config({
-- 	virtual_text = false,
-- 	signs = true,
-- 	update_in_insert = true,
-- 	underline = true,
-- 	severity_sort = false,
-- 	float = { border = "rounded", source = "always", header = "", prefix = "" },
-- })

vim.opt.completeopt = { "menuone", "noselect", "noinsert" }
vim.opt.shortmess = vim.opt.shortmess + { c = true }
vim.api.nvim_set_option("updatetime", 200)
vim.cmd([[ highlight! default link CmpItemKind CmpItemMenuDefault ]])

--- Enable floating window for diagnostics
-- vim.cmd([[ autocmd! CursorHold,CursorHoldI * lua vim.diagnostic.open_float(nil, { focus=false }) ]])
