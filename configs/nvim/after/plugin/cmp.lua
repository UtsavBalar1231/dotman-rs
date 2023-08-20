local status_ok, cmp = pcall(require, "cmp")
local status_ok_lspkind, lspkind = pcall(require, "lspkind")
local status_ok_luasnip, luasnip = pcall(require, "luasnip")

if not status_ok then
	return
end

if not status_ok_lspkind then
	return
end

if not status_ok_luasnip then
	return
end

cmp.setup({
	enabled = function()
		-- disable completion in comments
		local context = require("cmp.config.context")
		-- keep command mode completion enabled when cursor is in a comment
		if vim.api.nvim_get_mode().mode == "c" then
			return true
		else
			return not context.in_treesitter_capture("comment") and not context.in_syntax_group("Comment")
		end
	end,

	formatting = {
		fields = { "kind", "abbr", "menu" },
		format = function(entry, vim_item)
			if entry.source.name == "copilot" then
				vim_item.kind = "[] Copilot"
				vim_item.kind_hl_group = "CmpItemKindCopilot"
				return vim_item
			end
			-- fancy icons and a name of kind
			vim_item.kind = lspkind.presets.default[vim_item.kind] .. " " .. vim_item.kind

			-- set a name for each source
			vim_item.menu = ({
				path = "[Path]",
				nvim_lsp = "[LSP]",
				copilot = "[Copilot]",
				spell = "[Spell]",
				cmdline = "[CMD]",
				cmp_git = "[GIT]",
				luasnip = "[LuaSnip]",
				nvim_lua = "[NLua]",
				buffer = "[Buffer]",
			})[entry.source.name]
			return vim_item
		end,
	},
	preselect = cmp.PreselectMode.None,
	snippet = {
		expand = function(args)
			luasnip.lsp_expand(args.body)
		end,
	},
	completion = {
		keyword_length = 1,
	},
	experimental = {
		ghost_text = true,
	},
	duplicates = {
		nvim_lsp = 1,
		luasnip = 1,
		buffer = 1,
		path = 1,
	},
	confirm_opts = {
		behavior = cmp.ConfirmBehavior.Replace,
		select = false,
	},
	window = {
		highlight_hovered_item = true,
		highlight_selected_item = true,
	},
	mapping = cmp.mapping.preset.insert({
		-- snippet movement with luasnip
		["<Tab>"] = cmp.mapping(function(fallback)
			if cmp.visible() then
				cmp.select_next_item()
			elseif luasnip.expand_or_jumpable() then
				luasnip.expand_or_jump()
			else
				fallback()
			end
		end, { "i", "s" }),
		["<S-Tab>"] = cmp.mapping(function(fallback)
			if cmp.visible() then
				cmp.select_prev_item()
			elseif luasnip.jumpable(-1) then
				luasnip.jump(-1)
			else
				fallback()
			end
		end, { "i", "s" }),

		["<CR>"] = cmp.mapping({
			i = cmp.mapping.confirm({ behavior = cmp.ConfirmBehavior.Replace, select = false }),
			c = function(fallback)
				if cmp.visible() then
					cmp.confirm({ behavior = cmp.ConfirmBehavior.Replace, select = false })
				else
					fallback()
				end
			end,
		}, { "i", "c" }),
		["<C-Up>"] = cmp.mapping(cmp.mapping.scroll_docs(-4), { "i", "c" }),
		["<C-Down>"] = cmp.mapping(cmp.mapping.scroll_docs(4), { "i", "c" }),
	}),

	-- Installed sources
	sources = {
		{ name = "crates", priority = 250 },
		{ name = "path", priority = 250 },
		{ name = "buffer", keyword_length = 2, priority = 500 },
		{ name = "luasnip", priority = 750 },
		{ name = "nvim_lsp", priority = 1000 },
	},

	cmp.setup.filetype({ "gitcommit" }, {
		sources = {
			{ name = "cmp_git" }, -- You can specify the `cmp_git` source if you were installed it.
			{ name = "buffer" },
		},
	}),

	cmp.setup.filetype({ "toml", "rs" }, {
		sources = {
			{ name = "luasnip" },
			{ name = "crates" },
		},
	})
})

-- have a fixed column for the diagnostics to appear in
-- this removes the jitter when warnings/errors flow in
vim.wo.signcolumn = "yes"
vim.opt.shortmess = vim.opt.shortmess + { c = true }
vim.api.nvim_set_option("updatetime", 200)
vim.cmd([[ highlight! default link CmpItemKind CmpItemMenuDefault ]])

-- cmp highlights (gruvbox)
vim.cmd([[ highlight! CmpItemAbbrDeprecated guibg=NONE gui=strikethrough guifg=#7c6f64 ]])
vim.cmd([[ highlight! CmpItemAbbrMatch guibg=NONE guifg=#458588 ]])
vim.cmd([[ highlight! link CmpItemAbbrMatchFuzzy CmpItemAbbrMatch ]])
vim.cmd([[ highlight! CmpItemKindVariable guibg=NONE guifg=#83A598 ]])
vim.cmd([[ highlight! link CmpItemKindInterface CmpItemKindVariable ]])
vim.cmd([[ highlight! link CmpItemKindText CmpItemKindVariable ]])
vim.cmd([[ highlight! CmpItemKindFunction guibg=NONE guifg=#D3869B ]])
vim.cmd([[ highlight! link CmpItemKindMethod CmpItemKindFunction ]])
vim.cmd([[ highlight! CmpItemKindKeyword guibg=NONE guifg=#EBDBB2 ]])
vim.cmd([[ highlight! link CmpItemKindProperty CmpItemKindKeyword ]])
vim.cmd([[ highlight! link CmpItemKindUnit CmpItemKindKeyword ]])

--- Enable floating window for diagnostics
--- Map to <C-e> to toggle
vim.api.nvim_set_keymap(
	"n",
	"<leader>e",
	"<cmd>lua vim.diagnostic.open_float(0, { focus=false })<CR>",
	{ noremap = true, silent = true }
)

-- vim.cmd([[ autocmd! CursorHold,CursorHoldI * lua vim.diagnostic.open_float(nil, { focus=false }) ]])
