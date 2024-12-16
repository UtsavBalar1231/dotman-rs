return {
	"folke/which-key.nvim",
	event = "VeryLazy",
	opts = {
		layout = {
			align = "center",
		},
	},
	win = {
		border = "double",
		title = false,
		padding = { 1, 1 },
		no_overlap = true,
	},
	keys = {
		{
			"<leader>?",
			function()
				require("which-key").show({ global = false })
			end,
			desc = "Buffer Local Keymaps (which-key)",
		},
	},
}
