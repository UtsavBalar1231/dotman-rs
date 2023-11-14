-- Create/get autocommand group
local augroup = vim.api.nvim_create_augroup
-- Create autocommand
local autocmd = vim.api.nvim_create_autocmd
local utils = require("catDaddy.utils")

-- URL Highlighting on startup
vim.on_key(function(char)
	if vim.fn.mode() == "n" then
		local new_hlsearch = vim.tbl_contains({ "<CR>", "n", "N", "*", "#", "?", "/" }, vim.fn.keytrans(char))
		if vim.opt.hls:get() ~= new_hlsearch then
			vim.opt.hlsearch = new_hlsearch
		end
	end
end, vim.api.nvim_create_namespace("auto_hlsearch"))

autocmd({ "VimEnter", "FileType", "BufEnter", "WinEnter" }, {
	desc = "URL Highlighting",
	group = augroup("HighlightURL", { clear = true }),
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
