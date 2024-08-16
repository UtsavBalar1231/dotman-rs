local status_ok, indent_blankline = pcall(require, "ibl")

if not status_ok then
	vim.notify("Missing indent_blankline.nvim plugin", vim.log.levels.WARNING)
	return
end

indent_blankline.setup({
	scope = { show_start = false, show_end = false },
	exclude = {
		buftypes = {
			"nofile",
			"prompt",
			"quickfix",
			"terminal",
		},
		filetypes = {
			"aerial",
			"alpha",
			"dashboard",
			"help",
			"lazy",
			"mason",
			"neo-tree",
			"NvimTree",
			"neogitstatus",
			"notify",
			"startify",
			"toggleterm",
			"Trouble",
		},
	},
})
