-- Git gutter keymaps
vim.api.nvim_set_keymap("n", "<leader>g<Down>", ":GitGutterNextHunk<CR>", { noremap = true, silent = true })
vim.api.nvim_set_keymap("n", "<leader>g<Up>", ":GitGutterPrevHunk<CR>", { noremap = true, silent = true })
vim.api.nvim_set_keymap("n", "<leader>gs", ":GitGutterStageHunk<CR>", { noremap = true, silent = true })
vim.api.nvim_set_keymap("n", "<leader>gu", ":GitGutterUndoHunk<CR>", { noremap = true, silent = true })
vim.api.nvim_set_keymap("n", "<leader>gp", ":GitGutterPreviewHunk<CR>", { noremap = true, silent = true })
vim.api.nvim_set_keymap("n", "<leader>gR", ":GitGutterRevertHunk<CR>", { noremap = true, silent = true })
vim.api.nvim_set_keymap("n", "<leader>gdf", ":GitGutterDiffOrig<CR>", { noremap = true, silent = true })
