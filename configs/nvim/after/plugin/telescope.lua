local status_ok, telescope = pcall(require, "telescope")

if not status_ok then
	vim.notify("Missing telescope.nvim plugin", vim.log.levels.WARNING)
	return
end

-- Telescope Plugin Setup
local builtin = require("telescope.builtin")

-- Grep in current directory
local function telescope_buffer_dir()
	return vim.fn.expand("%:p:h")
end

vim.keymap.set("n", "<leader>fi", builtin.find_files, {})
vim.keymap.set("n", "<leader>fg", builtin.live_grep, {})

vim.keymap.set("n", "<leader>fb", builtin.buffers, {})
vim.keymap.set("n", "<leader>fh", builtin.help_tags, {})
vim.keymap.set("n", "<leader>fc", builtin.commands, {})
vim.keymap.set("n", "<leader>fo", builtin.oldfiles, {})
vim.keymap.set("n", "<leader>ft", builtin.tags, {})
vim.keymap.set("n", "<leader>f/", builtin.current_buffer_fuzzy_find, {})
vim.keymap.set("n", "<leader>f?", builtin.current_buffer_tags, {})
vim.keymap.set("n", "<leader>fe", builtin.diagnostics, {})
vim.keymap.set("n", "<leader>fq", builtin.quickfix, {})

-- Map <leader>f[ to goto previous diagnostic
vim.keymap.set("n", "<leader>f[", function()
	vim.diagnostic.goto_prev({ popup_opts = { border = "rounded" } })
end, {})

-- Map <leader>f] to goto next diagnostic
vim.keymap.set("n", "<leader>f]", function()
	vim.diagnostic.goto_next({ popup_opts = { border = "rounded" } })
end, {})

vim.keymap.set("n", "<leader>fd", builtin.lsp_definitions, {})
vim.keymap.set("n", "<leader>fw", builtin.lsp_dynamic_workspace_symbols, {})
vim.keymap.set("n", "<leader>fr", builtin.lsp_references, {})

-- git commands
vim.keymap.set("n", "<leader>gs", builtin.git_status, {})
vim.keymap.set("n", "<leader>gb", builtin.git_branches, {})
vim.keymap.set("n", "<leader>gc", builtin.git_commits, {})
vim.keymap.set("n", "<leader>gC", builtin.git_bcommits, {})
vim.keymap.set("n", "<leader>gf", builtin.git_files, {})
vim.keymap.set("n", "<leader>gS", builtin.git_stash, {})

vim.keymap.set("n", "ff", function()
	telescope.extensions.file_browser.file_browser({
		path = "%:p:h",
		cwd = telescope_buffer_dir(),
		respect_gitignore = false,
		hidden = true,
		grouped = true,
		previewer = true,
		initial_mode = "normal",
	})
end)

local fb_actions = require("telescope").extensions.file_browser.actions

telescope.setup({
	pickers = {
		find_files = {
			hidden = true,
			theme = "dropdown",
		},
	},
	defaults = {
	-- 	vimgrep_arguments = {
	-- 		"rg",
	-- 		"--color=never",
	-- 		"--no-heading",
	-- 		"--with-filename",
	-- 		"--line-number",
	-- 		"--column",
	-- 		"--smart-case",
	-- 	},
	-- 	prompt_prefix = "   ",
	-- 	selection_caret = "  ",
	-- 	entry_prefix = "  ",
	-- 	initial_mode = "insert",
	-- 	selection_strategy = "reset",
	-- 	sorting_strategy = "ascending",
	-- 	layout_strategy = "horizontal",
	-- 	layout_config = {
	-- 		horizontal = {
	-- 			prompt_position = "top",
	-- 			preview_width = 0.4,
	-- 			results_width = 0.8,
	-- 		},
	-- 		vertical = {
	-- 			mirror = false,
	-- 		},
	-- 		width = 0.87,
	-- 		height = 0.80,
	-- 		preview_cutoff = 160,
	-- 	},
		mappings = {
			i = {
				["<esc>"] = require("telescope.actions").close,
			},
			n = {
				["q"] = require("telescope.actions").close,
			},
		},
		file_sorter = require("telescope.sorters").get_fuzzy_file,
		file_ignore_patterns = { "node_modules" },
		generic_sorter = require("telescope.sorters").get_generic_fuzzy_sorter,
		path_display = { "truncate" },
		winblend = 0,
		border = {},
		-- borderchars = { "─", "│", "─", "│", "╭", "╮", "╯", "╰" },
		color_devicons = true,
		use_less = true,
		set_env = { ["COLORTERM"] = "truecolor" }, -- default = nil,
		file_previewer = require("telescope.previewers").vim_buffer_cat.new,
		grep_previewer = require("telescope.previewers").vim_buffer_vimgrep.new,
		qflist_previewer = require("telescope.previewers").vim_buffer_qflist.new,
		-- Developer configurations: Not meant for general override
		buffer_previewer_maker = require("telescope.previewers").buffer_previewer_maker,
	},
	extensions = {
		file_browser = {
			hijack_netrw = true,
			mappings = {
				["i"] = {
					["<C-w>"] = function()
						vim.cmd("normal vbd")
					end,
				},
				["n"] = {
					-- your custom normal mode mappings
					["N"] = fb_actions.create,
					["h"] = fb_actions.goto_parent_dir,
					["/"] = function()
						vim.cmd("startinsert")
					end,
				},
			},
			hide_dotfiles = false,
			show_hidden = true,
			file_sorter = require("telescope.sorters").get_fzy_sorter,
			file_ignore_patterns = {},
		},
	},
	frecency = {
		auto_validate = true,
	},
})

telescope.load_extension("file_browser")
telescope.load_extension("frecency")
