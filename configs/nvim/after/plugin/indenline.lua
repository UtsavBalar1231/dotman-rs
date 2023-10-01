local status_ok, indent_blankline = pcall(require, "ibl")

if not status_ok then
	vim.notify("Missing indent_blankline.nvim dependency", vim.log.levels.ERROR)
	return
end

indent_blankline.setup({})

