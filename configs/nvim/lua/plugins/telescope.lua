return {
	"nvim-telescope/telescope.nvim",
	dependencies = {
		"nvim-lua/plenary.nvim",
		"nvim-telescope/telescope.nvim",
		"nvim-telescope/telescope-file-browser.nvim",
		"nvim-telescope/telescope-ui-select.nvim",
		{ "nvim-telescope/telescope-fzf-native.nvim", build = "make" },
		"folke/trouble.nvim",
	},
	config = function()
		local telescope = require("telescope")
		-- local actions = require("telescope.actions")
		local builtin = require("telescope.builtin")

		telescope.load_extension("fzf")
		telescope.load_extension("file_browser")
		telescope.load_extension("ui-select")

		-- Grep in current directory
		local function telescope_buffer_dir()
			return vim.fn.expand("%:p:h")
		end

		vim.keymap.set("n", "<leader>fi", builtin.find_files, { desc = "Telescope find files" })
		vim.keymap.set("n", "<leader>fg", builtin.live_grep, { desc = "Telescope live grep" })

		-- Live Grep in parent directory of the current buffer
		local function parent_directory_of_current_file()
			-- Get the full path of the current file
			local current_file = vim.fn.expand("%:p")
			-- Get the parent directory of the current file
			return vim.fn.fnamemodify(current_file, ":h")
		end

		vim.keymap.set("n", "<leader>fG", function()
			builtin.live_grep({
				cwd = parent_directory_of_current_file(),
				desc = "Telescope live grep in parent directory",
			})
		end, {})

		vim.keymap.set("n", "<leader>fb", builtin.buffers, { desc = "Telescope buffers" })
		vim.keymap.set("n", "<leader>fh", builtin.help_tags, { desc = "Telescope help tags" })
		vim.keymap.set("n", "<leader>fc", builtin.commands, { desc = "Telescope commands" })
		vim.keymap.set("n", "<leader>fo", builtin.oldfiles, { desc = "Telescope old files" })
		vim.keymap.set("n", "<leader>ft", builtin.tags, { desc = "Telescope tags" })
		vim.keymap.set(
			"n",
			"<leader>f/",
			builtin.current_buffer_fuzzy_find,
			{ desc = "Telescope current buffer fuzzy find" }
		)
		vim.keymap.set("n", "<leader>f?", builtin.current_buffer_tags, { desc = "Telescope current buffer tags" })
		vim.keymap.set("n", "<leader>fe", builtin.diagnostics, { desc = "Telescope diagnostics" })
		vim.keymap.set("n", "<leader>fq", builtin.quickfix, { desc = "Telescope quickfix list" })

		-- Map <leader>f[ to goto previous diagnostic
		vim.keymap.set("n", "<leader>f[", function()
			vim.diagnostic.goto_prev({ popup_opts = { border = "rounded" }, desc = "Telescope diagnostics previous" })
		end, {})

		-- Map <leader>f] to goto next diagnostic
		vim.keymap.set("n", "<leader>f]", function()
			vim.diagnostic.goto_next({ popup_opts = { border = "rounded" }, desc = "Telescope diagnostics next" })
		end, {})

		vim.keymap.set("n", "<leader>fd", builtin.lsp_definitions, { desc = "Telescope lsp definitions" })
		vim.keymap.set(
			"n",
			"<leader>fw",
			builtin.lsp_dynamic_workspace_symbols,
			{ desc = "Telescope lsp dynamic workspace symbols" }
		)
		vim.keymap.set("n", "<leader>fr", builtin.lsp_references, { desc = "Telescope lsp references" })

		-- git commands
		vim.keymap.set("n", "<leader>gs", builtin.git_status, { desc = "Telescope git status" })
		vim.keymap.set("n", "<leader>gb", builtin.git_branches, { desc = "Telescope git branches" })
		vim.keymap.set("n", "<leader>gc", builtin.git_commits, { desc = "Telescope git commits" })
		vim.keymap.set("n", "<leader>gC", builtin.git_bcommits, { desc = "Telescope git buffer commits" })
		vim.keymap.set("n", "<leader>gf", builtin.git_files, { desc = "Telescope git files" })
		vim.keymap.set("n", "<leader>gS", builtin.git_stash, { desc = "Telescope git stash files" })

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
		local trouble_telescope = require("trouble.sources.telescope")

		telescope.setup({
			pickers = {
				find_files = {
					hidden = true,
					theme = "dropdown",
				},
			},
			defaults = {
				mappings = {
					i = {
						["<esc>"] = require("telescope.actions").close,
						["<c-t>"] = trouble_telescope.open,
					},
					n = {
						["q"] = require("telescope.actions").close,
						["<c-t>"] = trouble_telescope.open,
					},
				},
				file_sorter = require("telescope.sorters").get_fuzzy_file,
				file_ignore_patterns = { "node_modules" },
				generic_sorter = require("telescope.sorters").get_generic_fuzzy_sorter,
				path_display = { "truncate" },
				winblend = 1,
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
				wrap_results = true,
				fzf = {},
				["ui-select"] = {
					require("telescope.themes").get_dropdown({}),
				},
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
		})
	end,
}
