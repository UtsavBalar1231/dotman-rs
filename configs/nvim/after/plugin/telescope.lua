local status_ok, telescope = pcall(require, "telescope")

if not status_ok then
	return
end

-- Telescope Plugin Setup
local builtin = require("telescope.builtin")

vim.keymap.set("n", "<leader>fi", builtin.find_files, {})
vim.keymap.set("n", "<leader>fg", builtin.live_grep, {})

-- Grep in current directory
local function telescope_buffer_dir()
	return vim.fn.expand("%:p:h")
end
vim.keymap.set("n", "<leader>f ", function()
	builtin.live_grep({ search_dirs = { telescope_buffer_dir() } })
end, {})

vim.keymap.set("n", "<leader>fb", builtin.buffers, {})
vim.keymap.set("n", "<leader>fh", builtin.help_tags, {})
vim.keymap.set("n", "<leader>fc", builtin.commands, {})
vim.keymap.set("n", "<leader>fo", builtin.oldfiles, {})
vim.keymap.set("n", "<leader>ft", builtin.tags, {})
vim.keymap.set("n", "<leader>f/", builtin.current_buffer_fuzzy_find, {})
vim.keymap.set("n", "<leader>f?", builtin.current_buffer_tags, {})
vim.keymap.set("n", "<leader>fe", builtin.diagnostics, {})

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

local fb_actions = require("telescope").extensions.file_browser.actions

telescope.setup({
	defaults = {
		mappings = {
			i = {
				["<esc>"] = require("telescope.actions").close,
			},
		},
	},
	extensions = {
		file_browser = {
			hijack_netrw = true,
			mappings = {
				i = {
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

telescope.load_extension("file_browser")

-- Thiis is aa verry wrog commant
vim.keymap.set("n", "ff", function()
	telescope.extensions.file_browser.file_browser({
		path = "%:p:h",
		cwd = telescope_buffer_dir(),
		respect_gitignore = false,
		hidden = true,
		grouped = true,
		previewer = false,
		initial_mode = "normal",
		layout_config = { height = 40 },
	})
end)
