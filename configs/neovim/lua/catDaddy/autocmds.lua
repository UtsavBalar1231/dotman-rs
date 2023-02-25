-- Create/get autocommand group
local augroup = vim.api.nvim_create_augroup
-- Create autocommand
local autocmd = vim.api.nvim_create_autocmd

-- Highlight on yank
augroup("YankHighlight", {clear = true})
autocmd("TextYankPost", {
    group = "YankHighlight",
    callback = function()
        vim.highlight.on_yank({higroup = "IncSearch", timeout = "800"})
    end
})

-- Close nvim if NvimTree is only running buffer
autocmd("BufEnter", {
    command = [[if winnr('$') == 1 && bufname() == 'NvimTree_' . tabpagenr() | quit | endif]]
})

-- Remove whitespace on save
autocmd("BufWritePre", {pattern = "", command = ":%s/\\s\\+$//e"})

-- Don't auto commenting new lines
autocmd("BufEnter", {pattern = "", command = "set fo-=c fo-=r fo-=o"})

-- Set completeopt to have a better completion experience
autocmd("InsertEnter",
        {pattern = "", command = "setlocal completeopt=menuone,noselect"})

-- Avoid showing message extra message when using completion
autocmd("InsertLeave", {pattern = "", command = "setlocal completeopt=menuone"})

-- Avoid accidental writes to buffer that shouldn't be written
autocmd("BufReadPre", {pattern = "*.swp", command = "set noreadonly"})
autocmd("BufReadPre", {pattern = "*.bak", command = "set noreadonly"})
autocmd("BufReadPre", {pattern = "*.tmp", command = "set noreadonly"})
autocmd("BufReadPre", {pattern = "*.orig", command = "set noreadonly"})

-- Jump to last position when opening files
autocmd("BufReadPost", {
    pattern = "",
    command = [[if line("'\"") > 1 && line("'\"") <= line("$") | exe "normal! g`\"" | endif]]
})

-- Help with filetypes detection
autocmd("BufNewFile,BufRead",
        {pattern = "*.gitignore", command = "set filetype=gitignore"})
autocmd("BufNewFile,BufRead",
        {pattern = "*.md", command = "set filetype=markdown"})
autocmd("BufNewFile,BufRead", {pattern = "*.c", command = "set filetype=c"})
autocmd("BufNewFile,BufRead", {pattern = "*.h", command = "set filetype=c"})
autocmd("BufNewFile,BufRead", {pattern = "*.S", command = "set filetype=asm"})
autocmd("BufNewFile,BufRead", {pattern = "*.asm", command = "set filetype=asm"})
autocmd("BufNewFile,BufRead", {pattern = "*.s", command = "set filetype=asm"})
autocmd("BufNewFile,BufRead", {pattern = "*.sh", command = "set filetype=bash"})
autocmd("BufNewFile,BufRead", {pattern = "*.zsh", command = "set filetype=zsh"})
autocmd("BufNewFile,BufRead", {pattern = "*.lua", command = "set filetype=lua"})

-- https://github.com/hrsh7th/nvim-cmp/issues/519
-- Setting keyword_length 0 but making it only appear if only characters before cursor
-- autocmd(
--   {"TextChangedI", "TextChangedP"},
--   {
--     callback = function()
--       local line = vim.api.nvim_get_current_line()
--       local cursor = vim.api.nvim_win_get_cursor(0)[2]
--
--       local current = string.sub(line, cursor, cursor + 1)
--       if current == "." or current == "," or current == " " then
--         require('cmp').close()
--       end
--
--       local before_line = string.sub(line, 1, cursor + 1)
--       local after_line = string.sub(line, cursor + 1, -1)
--       if not string.match(before_line, '^%s+$') then
--         if after_line == "" or string.match(before_line, " $") or string.match(before_line, "%.$") then
--           require('cmp').complete()
--         end
--       end
--   end,
--   pattern = "*"
-- })
