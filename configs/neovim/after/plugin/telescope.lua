-- -- Telescope Plugin Setup
local builtin = require("telescope.builtin")
vim.keymap.set("n", "<leader>fi", builtin.find_files, {})

-- find files in current directory
vim.keymap.set("n", "<leader>ff", function()
  builtin.find_files({ cwd = vim.fn.expand("%:p:h") })
end, {})

vim.keymap.set("n", "<leader>fg", builtin.live_grep, {})
vim.keymap.set("n", "<leader>fb", builtin.buffers, {})
vim.keymap.set("n", "<leader>fh", builtin.help_tags, {})
vim.keymap.set("n", "<leader>fc", builtin.commands, {})
vim.keymap.set("n", "<leader>fo", builtin.oldfiles, {})
vim.keymap.set("n", "<leader>ft", builtin.tags, {})
vim.keymap.set("n", "<leader>f/", builtin.current_buffer_fuzzy_find, {})
vim.keymap.set("n", "<leader>f?", builtin.current_buffer_tags, {})

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

require("telescope").setup({
  defaults = {
	mappings = {
	  i = {
		["<esc>"] = require("telescope.actions").close,
	  },
	},
  },
})
