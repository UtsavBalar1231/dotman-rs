local status_ok, cmp = pcall(require, "cmp")

if not status_ok then
	return
end

local status_ok_lspkind, lspkind = pcall(require, "lspkind")

if not status_ok_lspkind then
	print("lspkind not found")
	return
end

local status_ok_luasnip, luasnip = pcall(require, "luasnip")

if not status_ok_luasnip then
	print("Luasnip not found")
	return
end

local window_opts = {
	winhighlight = "Normal:Normal,FloatBorder:FloatBorder,CursorLine:Visual,Search:None",
}

local function has_words_before()
	local line, col = table.unpack(vim.api.nvim_win_get_cursor(0))
	return col ~= 0 and vim.api.nvim_buf_get_lines(0, line - 1, line, true)[1]:sub(col, col):match("%s") == nil
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
		format = lspkind.cmp_format(),
	},
	preselect = cmp.PreselectMode.None,
	snippet = {
		expand = function(args)
			luasnip.lsp_expand(args.body)
		end,
	},
	duplicates = {
		nvim_lsp = 1,
		luasnip = 1,
		cmp_tabnine = 1,
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
			elseif has_words_before() then
				cmp.complete()
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

	--[[
	mapping = {
		["<C-Down>"] = cmp.mapping(cmp.mapping.select_next_item(), { "i", "c" }),
		["<C-Up>"] = cmp.mapping(cmp.mapping.select_prev_item(), { "i", "c" }),
		-- Add tab support
		["<S-Tab>"] = cmp.mapping.select_prev_item(),
		["<Tab>"] = cmp.mapping.select_next_item(),
		["<C-Space>"] = cmp.mapping(cmp.mapping.complete(), { "i", "c" }),
		["<C-e>"] = cmp.mapping({ i = cmp.mapping.close(), c = cmp.mapping.close() }),
			},
 ]]
	-- Installed sources
	sources = {
		{ name = "crates", priority = 250 },
		{ name = "path", priority = 250 },
		{ name = "buffer", keyword_length = 3, priority = 500 },
		{ name = "luasnip", priority = 750 },
		{ name = "nvim_lsp", priority = 1000 },
	},
})

-- have a fixed column for the diagnostics to appear in
-- this removes the jitter when warnings/errors flow in
vim.wo.signcolumn = "yes"
vim.opt.shortmess = vim.opt.shortmess + { c = true }
vim.api.nvim_set_option("updatetime", 200)
vim.cmd([[ highlight! default link CmpItemKind CmpItemMenuDefault ]])

-- Enable diagnostics
vim.lsp.handlers["textDocument/publishDiagnostics"] = vim.lsp.with(vim.lsp.diagnostic.on_publish_diagnostics, {
	virtual_text = false,
	digns = true,
	update_in_insert = false,
})

-- cmp highlights
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
-- vim.cmd([[ autocmd! CursorHold,CursorHoldI * lua vim.diagnostic.open_float(nil, { focus=false }) ]])
