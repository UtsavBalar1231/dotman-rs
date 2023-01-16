local keymap = vim.api.nvim_set_keymap
local default_options = { noremap = true, silent = true }

-- Map <leader>w to quick save
keymap("n", "<leader>w", ":w<CR>", default_options)

-- Map <leader>q to quick quit
keymap("n", "<leader>q", ":q<CR>", default_options)

-- Map <leader>Q to quick quit all
keymap("n", "<leader>Q", ":qa<CR>", default_options)

-- Map <leader>ss to save and source
keymap("n", "<leader>ss", ":w<CR>:source %<CR>", default_options)

-- Map <leader>ff to find files
keymap("n", "<leader>ff", ":Telescope find_files<CR>", default_options)
-- Map <leader>fg to find files in current directory
keymap("n", "<leader>fg", ":Telescope live_grep<CR>", default_options)
-- Map <leader>fb to find buffers
keymap("n", "<leader>fb", ":Telescope buffers<CR>", default_options)
-- Map <leader>fh to find help tags
keymap("n", "<leader>fh", ":Telescope help_tags<CR>", default_options)

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
vim.cmd("command! -bang Q q<bang>")
-- Use W to write bang
vim.cmd("command! -bang W w<bang>")
-- Use X to quit all bang
vim.cmd("command! -bang X x<bang>")

-- Centered search results
vim.cmd("nnoremap <silent> n nzz")
vim.cmd("nnoremap <silent> N Nzz")
vim.cmd("nnoremap <silent> * *zz")
vim.cmd("nnoremap <silent> # #zz")
vim.cmd("nnoremap <silent> g* g*zz")
vim.cmd("nnoremap <silent> g# g#zz")

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

-- Suspend with Ctrl-f
keymap("n", "<C-f>", ":suspend<CR>", default_options)
keymap("i", "<C-f>", "<Esc>:suspend<CR>", default_options)
keymap("v", "<C-f>", "<Esc>:suspend<CR>", default_options)

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

-- Open a new file adjacent to the current one
keymap("n", "<leader>o", ':e <C-R>=expand("%:p:h")."/"<CR>', default_options)

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
