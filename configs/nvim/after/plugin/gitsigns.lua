local status_ok, gitsigns = pcall(require, "gitsigns")

if not status_ok then
	return
end

gitsigns.setup({
	signs = {
		add = { hl = "GitSignsAdd", text = "▎", numhl = "GitSignsAddNr", linehl = "GitSignsAddLn" },
		change = { hl = "GitSignsChange", text = "▎", numhl = "GitSignsChangeNr", linehl = "GitSignsChangeLn" },
		delete = { hl = "GitSignsDelete", text = "契", numhl = "GitSignsDeleteNr", linehl = "GitSignsDeleteLn" },
		topdelete = { hl = "GitSignsDelete", text = "契", numhl = "GitSignsDeleteNr", linehl = "GitSignsDeleteLn" },
		changedelete = { hl = "GitSignsChange", text = "契", numhl = "GitSignsChangeNr", linehl = "GitSignsChangeLn" },
		untracked = { hl = "GitSignsAdd", text = "", numhl = "GitSignsAddNr", linehl = "GitSignsAddLn" },
	},
	numhl = true,
	linehl = false,

	on_attach = function(bufnr)
		local gs = package.loaded.gitsigns

		local function map(mode, l, r, opts)
			opts = opts or {}
			opts.buffer = bufnr
			vim.keymap.set(mode, l, r, opts)
		end
		-- Navigation
		map("n", "<leader>g<Down>", function()
			if vim.wo.diff then
				return "<leader>g<Down>"
			end
			vim.schedule(function()
				gs.next_hunk()
			end)
			return "<Ignore>"
		end, { expr = true })

		map("n", "<leader>g<Up>", function()
			if vim.wo.diff then
				return "<leader>g<Up>"
			end
			vim.schedule(function()
				gs.prev_hunk()
			end)
			return "<Ignore>"
		end, { expr = true })
		map("n", "<leader>hh", gs.preview_hunk)
		map("n", "<leader>gd", gs.diffthis)
		map("n", "<leader>gU", gs.undo_stage_hunk)
		map("n", "<leader>gS", gs.stage_buffer)
		map("n", "<leader>gR", gs.reset_buffer)
		map({ "o", "x" }, "ih", ":<C-U>Gitsigns select_hunk<CR>")
		map("n", "<leader>bl", function()
			gs.blame_line({ full = true })
		end)
		--[[ -- Actions
    map({'n', 'v'}, '<leader>hs', ':Gitsigns stage_hunk<CR>')
    map({'n', 'v'}, '<leader>hr', ':Gitsigns reset_hunk<CR>')
    map('n', '<leader>hD', function() gs.diffthis('~') end)
    map('n', '<leader>td', gs.toggle_deleted)
	map("n", "<leader>bl", gs.toggle_current_line_blame)

    -- Text object
	]]
	end,
})
