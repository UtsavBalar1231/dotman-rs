local keymap = vim.api.nvim_set_keymap

-- Map <leader>w to quick save
keymap("n", "<leader>w", "<cmd>:w<cr><esc>", { noremap = true, silent = true, desc = "Quick Save" })

-- Map <leader>Q to quick quit all
keymap("n", "<leader>Q", "<cmd>:qa<cr>", { noremap = true, silent = true, desc = "Quick Quit All" })

-- Map <leader>sw to save and source
keymap("n", "<leader>ss", "<cmd>:w<cr>:source %<cr>", { noremap = true, silent = true, desc = "Save and Source" })

-- Map <leader>tt to open a new tab
keymap("n", "<leader>tt", "<cmd>:tabnew<cr>", { noremap = true, silent = true, desc = "Open New Tab" })

-- Map <leader>tn to go to next tab
keymap("n", "<leader>tn", "<cmd>:tabnext<cr>", { noremap = true, silent = true, desc = "Next Tab" })

-- Map <leader>tp to go to previous tab
keymap("n", "<leader>tp", "<cmd>:tabprevious<cr>", { noremap = true, silent = true, desc = "Previous Tab" })

-- Map <leader>tw to close current tab
keymap("n", "<leader>tw", "<cmd>:tabclose<cr>", { noremap = true, silent = true, desc = "Close Current Tab" })

-- Map <leader>to to close all other tabs
keymap("n", "<leader>to", "<cmd>:tabonly<cr>", { noremap = true, silent = true, desc = "Close Other Tabs" })

-- Use Q to quit bang
keymap("n", "Q", "<cmd>:q<cr>", { noremap = true, silent = true, desc = "Quit" })

-- Use W to write bang
keymap("n", "W", "<cmd>:w<cr>", { noremap = true, silent = true, desc = "Write" })

-- Centered search results
keymap("n", "n", "nzz", { noremap = true, silent = true, desc = "Centered Search Next" })
keymap("n", "N", "Nzz", { noremap = true, silent = true, desc = "Centered Search Previous" })
keymap("n", "*", "*zz", { noremap = true, silent = true, desc = "Centered Search *" })
keymap("n", "#", "#zz", { noremap = true, silent = true, desc = "Centered Search #" })
keymap("n", "g*", "g*zz", { noremap = true, silent = true, desc = "Centered Search g*" })
keymap("n", "g#", "g#zz", { noremap = true, silent = true, desc = "Centered Search g#" })

-- Saner behavior of n and N
keymap("n", "n", "'Nn'[v:searchforward].'zv'", { expr = true, desc = "Next Search Result" })
keymap("x", "n", "'Nn'[v:searchforward]", { expr = true, desc = "Next Search Result" })
keymap("o", "n", "'Nn'[v:searchforward]", { expr = true, desc = "Next Search Result" })
keymap("n", "N", "'nN'[v:searchforward].'zv'", { expr = true, desc = "Previous Search Result" })
keymap("x", "N", "'nN'[v:searchforward]", { expr = true, desc = "Previous Search Result" })
keymap("o", "N", "'nN'[v:searchforward]", { expr = true, desc = "Previous Search Result" })

-- Ctrl+j and Ctrl+k as Esc
keymap("n", "<C-j>", "<Esc>", { noremap = true, silent = true, desc = "Escape" })
keymap("i", "<C-k>", "<Esc>", { noremap = true, silent = true, desc = "Escape" })
keymap("v", "<C-j>", "<Esc>", { noremap = true, silent = true, desc = "Escape" })
keymap("s", "<C-k>", "<Esc>", { noremap = true, silent = true, desc = "Escape" })
keymap("x", "<C-j>", "<Esc>", { noremap = true, silent = true, desc = "Escape" })
keymap("c", "<C-j>", "<Esc>", { noremap = true, silent = true, desc = "Escape" })
keymap("o", "<C-j>", "<Esc>", { noremap = true, silent = true, desc = "Escape" })
keymap("t", "<C-k>", "<Esc>", { noremap = true, silent = true, desc = "Escape" })

-- Clear search with <esc>
keymap("i", "<esc>", "<cmd>noh<cr><esc>", { noremap = true, silent = true, desc = "Clear Search" })
keymap("n", "<esc>", "<cmd>noh<cr><esc>", { noremap = true, silent = true, desc = "Clear Search" })

-- Clear search, diff update and redraw
keymap(
	"n",
	"<leader>ur",
	"<Cmd>nohlsearch<Bar>diffupdate<Bar>normal! <C-L><CR>",
	{ noremap = true, silent = true, desc = "Redraw / Clear Search / Diff Update" }
)

-- Jump to the end of the line with L
keymap("n", "L", "$", { noremap = true, silent = true, desc = "Jump to End of Line" })

-- Jump to the beginning of the line with H
keymap("n", "H", "^", { noremap = true, silent = true, desc = "Jump to Beginning of Line" })

-- Remap <C-a> to select all
keymap("n", "<C-a>", "ggVG", { noremap = true, silent = true, desc = "Select All" })

-- Proper X clipboard support
keymap("n", "<leader>y", '"+y', { noremap = true, silent = true, desc = "Copy to Clipboard" })
keymap("v", "<leader>y", '"+y', { noremap = true, silent = true, desc = "Copy to Clipboard" })
keymap("n", "<space>p", '"+p', { noremap = true, silent = true, desc = "Paste from Clipboard" })
keymap("v", "<space>p", '"+p', { noremap = true, silent = true, desc = "Paste from Clipboard" })

-- Left and right arrow keys to move between buffers
keymap("n", "<Left>", "<cmd>:bprevious<cr>", { noremap = true, silent = true, desc = "Previous Buffer" })
keymap("n", "<Right>", "<cmd>:bnext<cr>", { noremap = true, silent = true, desc = "Next Buffer" })
-- Swift navigation between buffers with leader + leader
keymap("n", "<leader><leader>", "<cmd>:b#<cr>", { noremap = true, silent = true, desc = "Previous Buffer" })

-- Hide or Show the invisible characters
keymap("n", "<leader>,", "<cmd>:set list!<cr>", { noremap = true, silent = true, desc = "Toggle Invisible Characters" })

-- <leader>x to chmod +x the current file
keymap("n", "<leader>x", "<cmd>!chmod +x %<cr>", { noremap = true, silent = true, desc = "Make File Executable" })

-- Keybindings for moving lines in normal mode
keymap("n", "<A-j>", "<cmd>:m .+1<cr>==", { noremap = true, silent = true, desc = "Move Line Down" })
keymap("n", "<A-k>", "<cmd>:m .-2<cr>==", { noremap = true, silent = true, desc = "Move Line Up" })
keymap("n", "<A-Down>", "<cmd>:m .+1<cr>==", { noremap = true, silent = true, desc = "Move Line Down" })
keymap("n", "<A-Up>", "<cmd>:m .-2<cr>==", { noremap = true, silent = true, desc = "Move Line Up" })

-- Keybindings for moving lines in insert mode
keymap("i", "<A-j>", "<Esc>:m .+1<cr>==gi", { noremap = true, silent = true, desc = "Move Line Down" })
keymap("i", "<A-k>", "<Esc>:m .-2<cr>==gi", { noremap = true, silent = true, desc = "Move Line Up" })
keymap("i", "<A-Down>", "<Esc>:m .+1<cr>==gi", { noremap = true, silent = true, desc = "Move Line Down" })
keymap("i", "<A-Up>", "<Esc>:m .-2<cr>==gi", { noremap = true, silent = true, desc = "Move Line Up" })

-- Keybindings for moving lines in visual mode
keymap("x", "<A-j>", "<cmd>:m '>+1<cr>gv=gv", { noremap = true, silent = true, desc = "Move Line Down" })
keymap("x", "<A-k>", "<cmd>:m '<-2<cr>gv=gv", { noremap = true, silent = true, desc = "Move Line Up" })
keymap("x", "<A-Down>", "<cmd>:m '>+1<cr>gv=gv", { noremap = true, silent = true, desc = "Move Line Down" })
keymap("x", "<A-Up>", "<cmd>:m '<-2<cr>gv=gv", { noremap = true, silent = true, desc = "Move Line Up" })

-- Update and install plugins
keymap("n", "<leader>uu", "<cmd>:Lazy sync<cr>", { noremap = true, silent = true, desc = "Packer Sync" })

-- Delete a buffer with the window
keymap("n", "<leader>dd", "<cmd>:bd<cr>", { noremap = true, silent = true, desc = "Delete Buffer" })

-- LSP code formatting
keymap(
	"n",
	"<leader>F",
	"<cmd>lua vim.lsp.buf.format { async = true } <cr>",
	{ noremap = true, silent = true, desc = "LSP Format" }
)
keymap(
	"v",
	"<Leader>1f",
	"<cmd>lua vim.lsp.buf.format { async = true } <cr>",
	{ noremap = true, silent = true, desc = "LSP Format" }
)

-- Map ss to split the current window horizontally
keymap("n", "sw", "<cmd>:split<cr><C-w>w", { noremap = true, silent = true, desc = "Horizontal Split" })
-- Map sv to split the current window vertically
keymap("n", "sv", "<cmd>:vsplit<cr><C-w>w", { noremap = true, silent = true, desc = "Vertical Split" })
-- Map sh to move to the left window
keymap("n", "sh", "<C-w>h", { noremap = true, silent = true, desc = "Move Left" })
-- Map sl to move to the right window
keymap("n", "sl", "<C-w>l", { noremap = true, silent = true, desc = "Move Right" })
-- Map sk to move to the top window
keymap("n", "sk", "<C-w>k", { noremap = true, silent = true, desc = "Move Up" })
-- Map sj to move to the bottom window
keymap("n", "sj", "<C-w>j", { noremap = true, silent = true, desc = "Move Down" })

-- Disable :help on <F1>
keymap("n", "<F1>", "<ESC>", { noremap = true, silent = true, desc = "Disable Help" })
keymap("i", "<F1>", "<ESC>", { noremap = true, silent = true, desc = "Disable Help" })

-- Set w!! to write the file with sudo permissions
keymap("c", "w!!", "w !sudo tee > /dev/null %", { noremap = true, silent = true, desc = "Write with Sudo" })

-- Set wq!! to write and quit with sudo permissions
keymap("c", "wq!!", "wq! !sudo tee > /dev/null %", { noremap = true, silent = true, desc = "Write and Quit with Sudo" })

-- better indenting
keymap("v", "<", "<gv", { noremap = true, silent = true, desc = "Better Indent" })
keymap("v", ">", ">gv", { noremap = true, silent = true, desc = "Better Indent" })

-- keymap to open lazy nvim
keymap("n", "<leader>l", "<cmd>Lazy<cr>", { noremap = true, silent = true, desc = "Lazy" })

-- keymap to open new file
keymap("n", "<leader>fn", "<cmd>enew<cr>", { noremap = true, silent = true, desc = "New File" })

-- keymaps for quickfix list
keymap("n", "<leader>xl", "<cmd>lopen<cr>", { desc = "Location List" })
keymap("n", "<leader>xq", "<cmd>copen<cr>", { desc = "Quickfix List" })
keymap("n", "[q", "lua vim.cmd.cprev()<cr>", { desc = "Previous Quickfix" })
keymap("n", "]q", "lua vim.cmd.cnext()<cr>", { desc = "Next Quickfix" })
