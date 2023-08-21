-- Create/get autocommand group
local augroup = vim.api.nvim_create_augroup
-- Create autocommand
local autocmd = vim.api.nvim_create_autocmd
local utils = require("catDaddy.utils")

-- URL Highlighting on startup
vim.on_key(function(char)
	if vim.fn.mode() == "n" then
		local new_hlsearch = vim.tbl_contains({ "<CR>", "n", "N", "*", "#", "?", "/" }, vim.fn.keytrans(char))
		if vim.opt.hlsearch:get() ~= new_hlsearch then
			vim.opt.hlsearch = new_hlsearch
		end
	end
end, vim.api.nvim_create_namespace("auto_hlsearch"))

autocmd({ "VimEnter", "FileType", "BufEnter", "WinEnter" }, {
	desc = "URL Highlighting",
	group = augroup("highlighturl", { clear = true }),
	pattern = "*",
	callback = function()
		utils.set_url_match()
	end,
})

-- Close the HELP, MAN, QUICKFIX, DAP FLOATS with q
autocmd("FileType", {
	desc = "Make q close help, man, quickfix, dap floats",
	group = augroup("q_close_windows", { clear = true }),
	pattern = { "qf", "help", "man", "dap-float" },
	callback = function(event)
		vim.keymap.set("n", "q", "<cmd>close<cr>", { buffer = event.buf, silent = true, nowait = true })
	end,
})

-- Unlist all the quickfix buffers
autocmd("FileType", {
	desc = "Unlist quickfist buffers",
	group = augroup("unlist_quickfist", { clear = true }),
	pattern = "qf",
	callback = function()
		vim.opt_local.buflisted = false
	end,
})

-- Highlight on yank
augroup("YankHighlight", { clear = true })
autocmd("TextYankPost", {
	desc = "Highlight on yank",
	group = "YankHighlight",
	callback = function()
		vim.highlight.on_yank({ higroup = "IncSearch", timeout = "400" })
	end,
})

-- Auto Close NVIM if only sidebars are open
autocmd("BufEnter", {
	desc = "Quit Nvim if more than one window is open and only sidebar windows are list",
	group = augroup("auto_quit", { clear = true }),
	callback = function()
		local wins = vim.api.nvim_tabpage_list_wins(0)
		-- Both neo-tree and aerial will auto-quit if there is only a single window left
		if #wins <= 1 then
			return
		end
		local sidebar_fts = { aerial = true, ["neo-tree"] = true }
		for _, winid in ipairs(wins) do
			if vim.api.nvim_win_is_valid(winid) then
				local bufnr = vim.api.nvim_win_get_buf(winid)
				local filetype = vim.api.nvim_get_option_value("filetype", { buf = bufnr })
				-- If any visible windows are not sidebars, early return
				if not sidebar_fts[filetype] then
					return
				-- If the visible window is a sidebar
				else
					-- only count filetypes once, so remove a found sidebar from the detection
					sidebar_fts[filetype] = nil
				end
			end
		end
		if #vim.api.nvim_list_tabpages() > 1 then
			vim.cmd.tabclose()
		else
			vim.cmd.qall()
		end
	end,
})

-- Remove whitespace on save
autocmd("BufWritePre", { pattern = "", command = ":%s/\\s\\+$//e" })

-- Set completeopt to have a better completion experience
autocmd("InsertEnter", { pattern = "", command = "setlocal completeopt=menuone,noselect" })

-- Avoid accidental writes to buffer that shouldn't be written
autocmd("BufReadPre", { pattern = "*.swp", command = "set noreadonly" })
autocmd("BufReadPre", { pattern = "*.bak", command = "set noreadonly" })
autocmd("BufReadPre", { pattern = "*.tmp", command = "set noreadonly" })
autocmd("BufReadPre", { pattern = "*.orig", command = "set noreadonly" })

-- Jump to last position when opening files
autocmd("BufReadPost", {
	pattern = "",
	command = [[if line("'\"") > 1 && line("'\"") <= line("$") | exe "normal! g`\"" | endif]],
})

-- Help with filetypes detection
autocmd("BufNewFile,BufRead", { pattern = "*.gitignore", command = "set filetype=gitignore" })
autocmd("BufNewFile,BufRead", { pattern = "*.md", command = "set filetype=markdown" })
autocmd("BufNewFile,BufRead", { pattern = "*.S", command = "set filetype=asm" })
autocmd("BufNewFile,BufRead", { pattern = "*.asm", command = "set filetype=asm" })
autocmd("BufNewFile,BufRead", { pattern = "*.s", command = "set filetype=asm" })
autocmd("BufNewFile,BufRead", { pattern = "*.sh", command = "set filetype=bash" })
autocmd("BufNewFile,BufRead", { pattern = "*.zsh", command = "set filetype=zsh" })
autocmd("BufNewFile,BufRead", { pattern = "*.lua", command = "set filetype=lua" })
