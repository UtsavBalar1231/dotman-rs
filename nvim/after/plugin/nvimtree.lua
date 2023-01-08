-- disable netrw at the very start of your init.lua (strongly advised)
vim.g.loaded_netrw = 1
vim.g.loaded_netrwPlugin = 1

-- set termguicolors to enable highlight groups
vim.opt.termguicolors = true

-- empty setup using defaults
require("nvim-tree").setup()

-- OR setup with some options
require("nvim-tree").setup({
	sort_by = "case_sensitive",
	view = {
		adaptive_size = true,
		mappings = {list = {{key = "u", action = "dir_up"}}}
	},
	renderer = {group_empty = true},
	filters = {dotfiles = true}
})

-- NvimTree Keybindings
vim.keymap.set('n', '<leader>nt',
function() require("nvim-tree.api").tree.toggle() end, {})
vim.keymap.set('n', '<leader>nr',
function() require("nvim-tree.api").tree.reload() end, {})
vim.keymap.set('n', '<leader>nd',
function() require("nvim-tree.api").tree.dir_up() end, {})
vim.keymap.set('n', '<leader>nc',
function() require("nvim-tree.api").tree.close() end, {})
vim.keymap.set('n', '<leader>nn',
function() require("nvim-tree.api").tree.create() end, {})

