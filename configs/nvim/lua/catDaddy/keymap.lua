local keymap = vim.api.nvim_set_keymap
local default_options = { noremap = true, silent = true }

-- Map <leader>w to quick save
keymap("n", "<leader>w", ":w<CR>", default_options)

-- Map <leader>Q to quick quit all
keymap("n", "<leader>Q", ":qa<CR>", default_options)

-- Map <leader>ss to save and source
keymap("n", "<leader>ss", ":w<CR>:source %<CR>", default_options)

-- Map <leader>tt to open a new tab
keymap("n", "<leader>tt", ":tabnew<CR>", default_options)
-- Map <leader>tn to go to next tab
keymap("n", "<leader>tn", ":tabnext<CR>", default_options)
-- Map <leader>tp to go to previous tab
keymap("n", "<leader>tp", ":tabprevious<CR>", default_options)
-- Map <leader>tc to close current tab
keymap("n", "<leader>tw", ":tabclose<CR>", default_options)
-- Map <leader>to to close all other tabs
keymap("n", "<leader>to", ":tabonly<CR>", default_options)

-- Use Q to quit bang
keymap("n", "Q", ":q<CR>", default_options)
-- Use W to write bang
keymap("n", "W", ":w<CR>", default_options)

-- Centered search results
keymap("n", "n", "nzz", default_options)
keymap("n", "N", "Nzz", default_options)
keymap("n", "*", "*zz", default_options)
keymap("n", "#", "#zz", default_options)
keymap("n", "g*", "g*zz", default_options)
keymap("n", "g#", "g#zz", default_options)

-- Ctrl+j and Ctrl+k as Esc
-- Ctrl-j is a little awkward unfortunately:
-- https://github.com/neovim/neovim/issues/5916
-- So we also map Ctrl+k
keymap("n", "<C-j>", "<Esc>", default_options)
keymap("n", "<C-j>", "<Esc>", default_options)

keymap("i", "<C-k>", "<Esc>", default_options)
keymap("i", "<C-k>", "<Esc>", default_options)

keymap("v", "<C-j>", "<Esc>", default_options)
keymap("v", "<C-j>", "<Esc>", default_options)

keymap("s", "<C-k>", "<Esc>", default_options)
keymap("s", "<C-k>", "<Esc>", default_options)

keymap("x", "<C-j>", "<Esc>", default_options)
keymap("x", "<C-k>", "<Esc>", default_options)

keymap("c", "<C-j>", "<Esc>", default_options)
keymap("c", "<C-j>", "<Esc>", default_options)

keymap("o", "<C-j>", "<Esc>", default_options)
keymap("o", "<C-k>", "<Esc>", default_options)

keymap("t", "<C-k>", "<Esc>", default_options)
keymap("t", "<C-k>", "<Esc>", default_options)

-- Remap <C-h> to stop highlighting search results
keymap("n", "<C-h>", ":noh<CR>", default_options)
keymap("v", "<C-h>", ":noh<CR>", default_options)

-- Remap <C-l> to clear search results
keymap("n", "<C-l>", ":nohlsearch<CR>", default_options)
keymap("v", "<C-l>", ":nohlsearch<CR>", default_options)

-- Jump to the end of the line with L (like in vim-easymotion)
keymap("n", "L", "$", default_options)

-- Jump to the beginning of the line with H (like in vim-easymotion)
keymap("n", "H", "^", default_options)

-- Remap <C-a> to select all
keymap("n", "<C-a>", "ggVG", default_options)

-- Proper X clipboard support
-- <leader>y to copy to clipboard
-- <space>p to paste from clipboard
keymap("n", "<leader>y", '"+y', default_options)
keymap("v", "<leader>y", '"+y', default_options)
keymap("n", "<space>p", '"+p', default_options)
keymap("v", "<space>p", '"+p', default_options)

-- Left and right arrow keys to move between buffers
keymap("n", "<Left>", ":bprevious<CR>", default_options)
keymap("n", "<Right>", ":bnext<CR>", default_options)

-- Swift navigation between buffers with leader + leader
keymap("n", "<leader><leader>", ":b#<CR>", default_options)

-- Hide or Show the invisible characters
keymap("n", "<leader>,", ":set list!<CR>", default_options)

-- <leader>x to chmod +x the current file
keymap("n", "<leader>x", "<cmd>!chmod +x %<CR>", { silent = true })

-- Move lines in visual line mode
keymap("v", "J", ":m '>+1<CR>gv=gv", default_options)
keymap("v", "K", ":m '<-2<CR>gv=gv", default_options)
keymap("v", "<leader><Down>", ":m '>+1<CR>gv=gv", default_options)
keymap("v", "<leader><Up>", ":m '<-2<CR>gv=gv", default_options)

-- Update and install plugins
keymap("n", "<leader>uu", ":PackerSync<CR>", default_options)

-- Delete a buffer without closing the window
keymap("n", "<leader>dd", ":bd<CR>", default_options)

-- LSP code formatting
keymap("n", "<leader>F", "<cmd>lua vim.lsp.buf.format { async = true } <CR>", default_options)

--- Window navigation {{{

-- Map ss to split the current window horizontally
keymap("n", "ss", ":split<CR><C-w>w", default_options)

-- Map sv to split the current window vertically
keymap("n", "sv", ":vsplit<CR><C-w>w", default_options)

-- Map sh to move to the left window
keymap("n", "sh", "<C-w>h", default_options)

-- Map sl to move to the right window
keymap("n", "sl", "<C-w>l", default_options)

-- Map sk to move to the top window
keymap("n", "sk", "<C-w>k", default_options)

-- Map sj to move to the bottom window
keymap("n", "sj", "<C-w>j", default_options)

--- Window navigation }}}

-- Disable :help on <F1>
keymap("n", "<F1>", "<ESC>", default_options)
keymap("i", "<F1>", "<ESC>", default_options)
