local status_ok, treesitter_configs = pcall(require, "nvim-treesitter.configs")

if not status_ok then
	return
end

-- Treesitter Plugin Setup Start --
treesitter_configs.setup({
	context_commentstring = {
		enable = true,
		enable_autocmd = false,
	},
	ensure_installed = {
		"bash",
		"c",
		"cpp",
		"go",
		"python",
		"rust",
		"vim",
		"json",
		"lua",
		"toml",
		"yaml",
		"html",
	},
	matchup = { enable = true },
	auto_install = true,
	highlight = { enable = true, additional_vim_regex_highlighting = false },
	ident = { enable = true },
	rainbow = { enable = true, extended_mode = true, max_file_lines = nil },
	incremental_selection = {
		enable = true,
	},
})

-- Treesittolding
-- vim.wo.foldmethod = 'expr'
-- vim.wo.foldexpr = 'nvim_treesitter#foldexpr()'
