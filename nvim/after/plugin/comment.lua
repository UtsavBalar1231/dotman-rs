require("Comment").setup({
	---Add a space b/w comment and the line
	padding = true,
	---Whether the cursor should stay at its position
	sticky = true,
	---LHS of toggle mappings in NORMAL mode
	toggler = {
		---Line-comment toggle keymap
		line = "<leader>cl",
		---Block-comment toggle keymap
		block = "<leader>cb",
	},
})
