return {
	"nvim-tree/nvim-tree.lua",
	cmd = { "NvimTreeToggle", "NvimTreeFocus" },
	dependencies = {
		"nvim-tree/nvim-web-devicons",
	},
	config = function()
		require("nvim-tree").setup({
			filters = { dotfiles = false },
			disable_netrw = true,
			hijack_cursor = true,
			sync_root_with_cwd = true,
			update_focused_file = {
				enable = true,
				update_root = false,
			},
			view = {
				width = 30,
				preserve_window_proportions = true,
			},
			renderer = {
				root_folder_label = false,
				highlight_git = true,
				indent_markers = { enable = true },
				icons = {
					glyphs = {
						default = "󰈚",
						folder = {
							default = "",
							empty = "",
							empty_open = "",
							open = "",
							symlink = "",
						},
						git = { unmerged = "" },
					},
				},
			},
		})

		local map = vim.keymap.set

		map("n", "<leader>q", "<cmd>NvimTreeToggle<CR>", { desc = "nvimtree toggle window" })
		map("n", "<leader>t", "<cmd>NvimTreeFocus<CR>", { desc = "nvimtree focus window" })
	end,
}
