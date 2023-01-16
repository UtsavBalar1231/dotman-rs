require("Comment").setup({
	---Add a space b/w comment and the line
	padding = true,
	---Whether the cursor should stay at its position
	sticky = true,
	---LHS of toggle mappings in NORMAL mode
	toggler = {
		---Line-comment toggle keymap
		line = "<leader>l",
		---Block-comment toggle keymap
		block = "<leader>b",
	},

	---LHS of operator-pending mappings in NORMAL/VISUAL mode
	operator = {
		---Line-comment keymap
		line = "gc",
		---Block-comment keymap
		block = "gb",
	},

	---LHS of textobject mappings in NORMAL mode
	textobject = {
		---Available textobjects
		---You can define your own textobjects here
		---NOTE: You must define a *complete* list of textobjects
		---      If you don't want to define a textobject, just set it to an empty table
		---      i.e. line = {}
		line = {
			---line-comment textobject keymap
			---NOTE: You can use the same keymap for multiple textobjects
			---      But you can't use the same keymap for the same textobject
			---      i.e. You can't have both line and block use the same keymap
			---      But you can have line and selection use the same keymap
			"<leader>cl",
		},
		block = {
			---block-comment textobject keymap
			"<leader>cb",
		},
		selection = {
			---selection-comment textobject keymap
			"<leader>cs",
		},
	},
})
