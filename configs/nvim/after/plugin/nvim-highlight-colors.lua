local status_ok, nvim_highlight_colors = pcall(require, "nvim-highlight-colors")

if not status_ok then
	vim.notify("Missing nvim_highlight_colors plugin", vim.log.levels.WARNING)
	return
end

nvim_highlight_colors.setup({})
