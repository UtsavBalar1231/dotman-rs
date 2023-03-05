-- Create/get autocommand group
local augroup = vim.api.nvim_create_augroup
-- Create autocommand
local autocmd = vim.api.nvim_create_autocmd

-- Highlight on yank
augroup("YankHighlight", { clear = true })
autocmd("TextYankPost", {
	group = "YankHighlight",
	callback = function()
		vim.highlight.on_yank({ higroup = "IncSearch", timeout = "400" })
	end,
})

-- Remove whitespace on save
autocmd("BufWritePre", { pattern = "", command = ":%s/\\s\\+$//e" })

-- Set completeopt to have a better completion experience
autocmd("InsertEnter", { pattern = "", command = "setlocal completeopt=menuone,noselect" })

-- Avoid showing message extra message when using completion
autocmd("InsertLeave", { pattern = "", command = "setlocal completeopt=menuone" })

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

-- Enable auto formatting on save
autocmd("BufWritePre", { pattern = "*.lua", command = "lua vim.lsp.buf.format { async = true }" })
autocmd("BufWritePre", { pattern = "*.sh", command = "lua vim.lsp.buf.format { async = true }" })
autocmd("BufWritePre", { pattern = "*.zsh", command = "lua vim.lsp.buf.format { async = true }" })
autocmd("BufWritePre", { pattern = "*.rs", command = "lua vim.lsp.buf.format { async = true }" })
