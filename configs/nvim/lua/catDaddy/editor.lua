local g = vim.g
local opt = vim.opt

-- Set ZSH as default global shell
g.shell = "/usr/bin/zsh"

-- Map the leader key to space
g.mapleader = " "

-- Enable auto indentation in vim
opt.autoindent = true

-- http://stackoverflow.com/questions/2158516/delay-before-o-opens-a-new-line
opt.timeoutlen = 200

-- Set default encoding
opt.encoding = "utf-8"

-- Default scrolloff in vim
opt.scrolloff = 4

-- Enable undo dir setup
opt.undodir = vim.fn.stdpath("config") .. "/../../.vimdid"
opt.undofile = true

-----------------------
--- Sane tabs setup ---
-----------------------
-- Do not use spaces for tabs
opt.expandtab = false
-- Shift 4 spaces when tab
opt.shiftwidth = 4
-- 1 tab == 4 spaces
opt.tabstop = 4
-- Autoindent new lines
opt.smartindent = true
-- Smart tab
opt.smarttab = true

---------------------------
--- Better search setup ---
---------------------------
-- Ignore case when searching
opt.ignorecase = true
-- But be smart about it
opt.smartcase = true
-- Highlight search results
opt.hlsearch = true
-- Incremental search
opt.incsearch = true
-- grep-like search
opt.gdefault = true

-------------------------------
--- General editor settings ---
-------------------------------
-- Enable mouse support
opt.mouse = "a"
-- Copy/paste to system clipboard
opt.clipboard:append("unnamedplus")
-- Autocomplete options
opt.completeopt = "menuone,noinsert,noselect"

-------------------------------
--- General editor UI setup ---
-------------------------------
-- Show line number
opt.number = true
-- Enable relative line numbers
opt.relativenumber = true
-- Highlight matching parenthesis
opt.showmatch = true
-- Enable folding (default 'foldmarker')
opt.foldmethod = "marker"
-- Line length marker at 120 columns
opt.colorcolumn = "80"
-- Vertical split to the right
opt.splitright = true
-- Horizontal split to the bottom
opt.splitbelow = true
-- Ignore case letters when search
opt.ignorecase = true
-- Ignore lowercase for the whole pattern
opt.smartcase = true
-- Wrap on word boundary
opt.linebreak = true
-- Enable 24-bit RGB colors
opt.termguicolors = true
-- Set global statusline
opt.laststatus = 3
-- Use backspaces over new line
opt.backspace = "2"
-- Enable ttyfast
opt.ttyfast = true
-- Show (partial) command in status line
opt.showcmd = true
-- No show mode
opt.showmode = false
-- Show nbsp, extends, precedes and trailing spaces
opt.list = false
opt.listchars = "nbsp:¬,extends:»,precedes:«,trail:•"
-- Better display for messages
opt.cmdheight = 1
-- Show cursor line
opt.cursorline = true

----------------------------
--- Format options setup ---
----------------------------
opt.formatoptions:append("n") -- Auto indent new lines
opt.formatoptions:append("q") -- Allow formatting comments w/ gq
opt.formatoptions:append("r") -- Auto indent after paste
opt.formatoptions:append("t") -- Auto indent after <C-t>
opt.formatoptions:append("c") -- Auto indent comment lines
opt.formatoptions:append("b") -- Auto indent after <C-o>

--- Make diffing better: https://vimways.org/2018/the-power-of-diff/
opt.diffopt:append("iwhite")
opt.diffopt:append("algorithm:patience")
opt.diffopt:append("indent-heuristic")
