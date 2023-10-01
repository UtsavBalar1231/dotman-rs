local status_ok, treesitter_configs = pcall(require, "nvim-treesitter.configs")

if not status_ok then
	vim.notify("Missing nvim-treesitter.configs", vim.log.levels.ERROR)
	return
end

-- Treesitter Plugin Setup Start --
treesitter_configs.setup({
	context_commentstring = {
		enable = true,
		enable_autocmd = true,
	},
	ensure_installed = {
		"bash",
		"c",
		"cpp",
		"go",
		"html",
		"json",
		"latex",
		"lua",
		"markdown",
		"markdown_inline",
		"python",
		"rust",
		"toml",
		"vim",
		"yaml",
	},
	matchup = { enable = true },
	auto_install = true,
	auto_tag = {
		enable = true,
		filetypes = { "html", "xml", "rust" },
	},
	highlight = {
		enable = true,
		additional_vim_regex_highlighting = { "latex" },
	},
	ident = { enable = true },
	rainbow = {
		enable = true,
		extended_mode = true,
		max_file_lines = nil,
	},
	incremental_selection = {
		enable = true,
	},
	textsubjects = {
		enable = true,
		prev_selection = ",",
		keymaps = {
			["."] = "textsubjects-smart",
			[";"] = "textsubjects-container-outer",
			["i;"] = "textsubjects-container-inner",
		},
	},
	textobjects = {
		lsp_interop = {
			enable = true,
			border = "none",
			peek_definition_code = {
				["<leader>]f"] = "@function.outer",
				["<leader>]F"] = "@class.outer",
			},
		},
		swap = {
			enable = true,
			swap_next = {
				["]s"] = "@parameter.inner",
			},
			swap_previous = {
				["[s"] = "@parameter.inner",
			},
		},
		move = {
			enable = true,
			set_jumps = true, -- whether to set jumps in the jumplist
			goto_next_start = {
				["]m"] = "@function.outer",
				["]]"] = "@class.outer",
				["]a"] = "@parameter.inner",
			},
			goto_next_end = {
				["]M"] = "@function.outer",
				["]["] = "@class.outer",
				["]A"] = "@parameter.inner",
			},
			goto_previous_start = {
				["[m"] = "@function.outer",
				["[["] = "@class.outer",
				["[a"] = "@parameter.inner",
			},
			goto_previous_end = {
				["[M"] = "@function.outer",
				["[]"] = "@class.outer",
				["[A"] = "@parameter.inner",
			},
		},
		select = {
			enable = true,
			lookahead = true,
			keymaps = {
				-- You can use the capture groups defined in textobjects.scm
				["af"] = "@function.outer",
				["aa"] = "@parameter.outer",
				["ia"] = "@parameter.inner",
				["if"] = "@function.inner",
				["ac"] = "@class.outer",
				["ic"] = "@class.inner",
			},
		},
	},
})

require("treesitter-context").setup({
	enable = true,
	max_lines = 0,
	min_window_height = 0,
	line_numbers = true,
	multiline_threshold = 20,
	trim_scope = "outer",
	mode = "cursor",
	separator = nil,
	zindex = 20,
	on_attach = nil,
})
