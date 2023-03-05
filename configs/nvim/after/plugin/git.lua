local status_ok, git = pcall(require, "git")

if not status_ok then
	return
end

git.setup({
	default_mappings = true,

	keymaps = {
		-- Open blame window
		blame = "<Leader>gb",
		-- Close blame window
		quit_blame = "q",
		-- Open blame commit
		blame_commit = "<CR>",
		-- Open file/folder in git repository
		browse = "<Leader>go",
		-- Open pull request of the current branch
		open_pull_request = "<Leader>gp",
		-- Create a pull request with the target branch is set in the `target_branch` option
		create_pull_request = "<Leader>gn",
		-- Opens a new diff that compares against the current index
		diff = "<Leader>gd",
		-- Close git diff
		diff_close = "<Leader>gD",
		-- Revert to the specific commit
		revert = "<Leader>gr",
		-- Revert the current file to the specific commit
		revert_file = "<Leader>gR",
	},
	-- Default target branch when create a pull request
	target_branch = "master",
})

-- Git gutter keymaps
--[[ vim.api.nvim_set_keymap("n", "<leader>g<Down>", ":GitGutterNextHunk<CR>", { noremap = true, silent = true })
vim.api.nvim_set_keymap("n", "<leader>g<Up>", ":GitGutterPrevHunk<CR>", { noremap = true, silent = true })
vim.api.nvim_set_keymap("n", "<leader>gs", ":GitGutterStageHunk<CR>", { noremap = true, silent = true })
vim.api.nvim_set_keymap("n", "<leader>gu", ":GitGutterUndoHunk<CR>", { noremap = true, silent = true })
vim.api.nvim_set_keymap("n", "<leader>gp", ":GitGutterPreviewHunk<CR>", { noremap = true, silent = true })
vim.api.nvim_set_keymap("n", "<leader>gR", ":GitGutterRevertHunk<CR>", { noremap = true, silent = true })
vim.api.nvim_set_keymap("n", "<leader>gdf", ":GitGutterDiffOrig<CR>", { noremap = true, silent = true }) ]]
