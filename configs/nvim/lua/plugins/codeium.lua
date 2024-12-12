return {
	"Exafunction/codeium.vim",
	dependencies = {
		"nvim-lua/plenary.nvim",
		"hrsh7th/nvim-cmp",
	},
	event = 'BufEnter',
	config = function()

		vim.keymap.set('i', '<C-g>', function () return vim.fn['codeium#Accept']() end, { expr = true, silent = true, desc = "Codeium Accept Completion" })
		vim.keymap.set('i', '<C-,>', function() return vim.fn['codeium#CycleCompletions'](1) end, { expr = true, silent = true, desc = "Codeium Cycle Completion right" })
		vim.keymap.set('i', '<C-.>', function() return vim.fn['codeium#CycleCompletions'](-1) end, { expr = true, silent = true, desc = "Codeium Cycle Completion left" })
		vim.keymap.set('i', '<C-x>', function() return vim.fn['codeium#Clear']() end, { expr = true, silent = true, desc = "Codeium clear" })
	end
}
