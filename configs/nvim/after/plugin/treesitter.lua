local status_ok, treesitter_configs = pcall(require, "nvim-treesitter.configs")

if not status_ok then
	vim.notify("Missing nvim-treesitter.configs plugin", vim.log.levels.WARNING)
	return
end

-- Treesitter Plugin Setup Start --
treesitter_configs.setup({
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
	matchup = { enable = false },
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

local status_treesitter_context, treesitter_context = pcall(require, "treesitter-context")

if not status_treesitter_context then
	vim.notify("Missing treesitter context plugin", vim.log.levels.WARNING)
else
	treesitter_context.setup({
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
end

local status_ts_context_commentstring, ts_context_commentstring = pcall(require, "ts_context_commentstring")

if not status_ts_context_commentstring then
	vim.notify("Missing ts_context_commentstring plugin", vim.log.levels.WARNING)
else
	ts_context_commentstring.setup({
		enable_autocmd = true,
	})

	vim.g.skip_ts_context_comment = true
end

