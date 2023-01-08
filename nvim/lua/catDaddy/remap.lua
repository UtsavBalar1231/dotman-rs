-- Map <leader>w to quick save
vim.api.nvim_set_keymap('n', '<leader>w', ':w<CR>',
{noremap = true, silent = true})

-- Map <leader>q to quick quit
vim.api.nvim_set_keymap('n', '<leader>q', ':q<CR>',
{noremap = true, silent = true})

-- Map <leader>Q to quick quit all
vim.api.nvim_set_keymap('n', '<leader>Q', ':qa<CR>',
{noremap = true, silent = true})

-- Map <leader>ss to save and source
vim.api.nvim_set_keymap('n', '<leader>ss', ':w<CR>:source %<CR>',
{noremap = true, silent = true})

-- Map <leader>ff to find files
vim.api.nvim_set_keymap('n', '<leader>ff', ':Telescope find_files<CR>',
{noremap = true, silent = true})
-- Map <leader>fg to find files in current directory
vim.api.nvim_set_keymap('n', '<leader>fg', ':Telescope live_grep<CR>',
{noremap = true, silent = true})
-- Map <leader>fb to find buffers
vim.api.nvim_set_keymap('n', '<leader>fb', ':Telescope buffers<CR>',
{noremap = true, silent = true})
-- Map <leader>fh to find help tags
vim.api.nvim_set_keymap('n', '<leader>fh', ':Telescope help_tags<CR>',
{noremap = true, silent = true})

-- Map <leader>tt to open a new tab
vim.api.nvim_set_keymap('n', '<leader>tt', ':tabnew<CR>',
{noremap = true, silent = true})
-- Map <leader>tn to go to next tab
vim.api.nvim_set_keymap('n', '<leader>tn', ':tabnext<CR>',
{noremap = true, silent = true})
-- Map <leader>tp to go to previous tab
vim.api.nvim_set_keymap('n', '<leader>tp', ':tabprevious<CR>',
{noremap = true, silent = true})
-- Map <leader>tc to close current tab
vim.api.nvim_set_keymap('n', '<leader>tw', ':tabclose<CR>',
{noremap = true, silent = true})
-- Map <leader>to to close all other tabs
vim.api.nvim_set_keymap('n', '<leader>to', ':tabonly<CR>',
{noremap = true, silent = true})

-- Use Q to quit bang
vim.cmd('command! -bang Q q<bang>')
-- Use W to write bang
vim.cmd('command! -bang W w<bang>')
-- Use X to quit all bang
vim.cmd('command! -bang X x<bang>')

-- Centered search results
vim.cmd('nnoremap <silent> n nzz')
vim.cmd('nnoremap <silent> N Nzz')
vim.cmd('nnoremap <silent> * *zz')
vim.cmd('nnoremap <silent> # #zz')
vim.cmd('nnoremap <silent> g* g*zz')
vim.cmd('nnoremap <silent> g# g#zz')

-- Remap ; as :
vim.api.nvim_set_keymap('n', ';', ':', {noremap = true, silent = true})

-- Ctrl+j and Ctrl+k as Esc
-- Ctrl-j is a little awkward unfortunately:
-- https://github.com/neovim/neovim/issues/5916
-- So we also map Ctrl+k
vim.api.nvim_set_keymap('n', '<C-j>', '<Esc>', {noremap = true, silent = true})
vim.api.nvim_set_keymap('n', '<C-j>', '<Esc>', {noremap = true, silent = true})

vim.api.nvim_set_keymap('i', '<C-k>', '<Esc>', {noremap = true, silent = true})
vim.api.nvim_set_keymap('i', '<C-k>', '<Esc>', {noremap = true, silent = true})

vim.api.nvim_set_keymap('v', '<C-j>', '<Esc>', {noremap = true, silent = true})
vim.api.nvim_set_keymap('v', '<C-j>', '<Esc>', {noremap = true, silent = true})

vim.api.nvim_set_keymap('s', '<C-k>', '<Esc>', {noremap = true, silent = true})
vim.api.nvim_set_keymap('s', '<C-k>', '<Esc>', {noremap = true, silent = true})

vim.api.nvim_set_keymap('x', '<C-j>', '<Esc>', {noremap = true, silent = true})
vim.api.nvim_set_keymap('x', '<C-k>', '<Esc>', {noremap = true, silent = true})

vim.api.nvim_set_keymap('c', '<C-j>', '<Esc>', {noremap = true, silent = true})
vim.api.nvim_set_keymap('c', '<C-j>', '<Esc>', {noremap = true, silent = true})

vim.api.nvim_set_keymap('o', '<C-j>', '<Esc>', {noremap = true, silent = true})
vim.api.nvim_set_keymap('o', '<C-k>', '<Esc>', {noremap = true, silent = true})

vim.api.nvim_set_keymap('t', '<C-k>', '<Esc>', {noremap = true, silent = true})
vim.api.nvim_set_keymap('t', '<C-k>', '<Esc>', {noremap = true, silent = true})

-- Remap <C-h> to stop highlighting search results
vim.api.nvim_set_keymap('n', '<C-h>', ':noh<CR>',
{noremap = true, silent = true})
vim.api.nvim_set_keymap('v', '<C-h>', ':noh<CR>',
{noremap = true, silent = true})

-- Remap <C-l> to clear search results
vim.api.nvim_set_keymap('n', '<C-l>', ':nohlsearch<CR>',
{noremap = true, silent = true})
vim.api.nvim_set_keymap('v', '<C-l>', ':nohlsearch<CR>',
{noremap = true, silent = true})

-- Suspend with Ctrl-f
vim.api.nvim_set_keymap('n', '<C-f>', ':suspend<CR>',
{noremap = true, silent = true})
vim.api.nvim_set_keymap('i', '<C-f>', '<Esc>:suspend<CR>',
{noremap = true, silent = true})
vim.api.nvim_set_keymap('v', '<C-f>', '<Esc>:suspend<CR>',
{noremap = true, silent = true})

-- Jump to the end of the line with L (like in vim-easymotion)
vim.api.nvim_set_keymap('n', 'L', '$', {noremap = true, silent = true})

-- Jump to the beginning of the line with H (like in vim-easymotion)
vim.api.nvim_set_keymap('n', 'H', '^', {noremap = true, silent = true})

-- Remap <C-a> to select all
vim.api.nvim_set_keymap('n', '<C-a>', 'ggVG', {noremap = true, silent = true})

-- Proper X clipboard support
-- <leader>y to copy to clipboard
-- <space>p to paste from clipboard
vim.api
.nvim_set_keymap('n', '<leader>y', '"+y', {noremap = true, silent = true})
vim.api
.nvim_set_keymap('v', '<leader>y', '"+y', {noremap = true, silent = true})
vim.api.nvim_set_keymap('n', '<space>p', '"+p', {noremap = true, silent = true})
vim.api.nvim_set_keymap('v', '<space>p', '"+p', {noremap = true, silent = true})

-- Open a new file adjacent to the current one
vim.api.nvim_set_keymap('n', '<leader>o', ':e <C-R>=expand("%:p:h")."/"<CR>',
{noremap = true, silent = true})

-- Left and right arrow keys to move between buffers
vim.api.nvim_set_keymap('n', '<Left>', ':bprevious<CR>',
{noremap = true, silent = true})
vim.api.nvim_set_keymap('n', '<Right>', ':bnext<CR>',
{noremap = true, silent = true})

-- Swift navigation between buffers with leader + leader
vim.api.nvim_set_keymap('n', '<leader><leader>', ':b#<CR>',
{noremap = true, silent = true})

-- Hide or Show the invisible characters
vim.api.nvim_set_keymap('n', '<leader>,', ':set list!<CR>',
{noremap = true, silent = true})

-- <leader>x to chmod +x the current file
vim.api.nvim_set_keymap('n', '<leader>x', '<cmd>!chmod +x %<CR>',
{silent = true})
