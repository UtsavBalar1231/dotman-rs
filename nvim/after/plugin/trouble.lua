-- trouble Plugin Setup
require("trouble").setup({
	auto_open = false,
	auto_close = true,
	auto_preview = false,
	auto_fold = false,
	use_diagnostic_signs = true,
})

-- Map <leader>tr to open trouble
vim.api.nvim_set_keymap("n", "<leader>tr", "<cmd>TroubleToggle<cr>", { noremap = true, silent = true })
