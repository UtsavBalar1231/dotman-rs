set shell=/usr/bin/zsh
let mapleader = "\<Space>"
let g:loaded_matchit = 1
let g:loaded_matchparen = 1
" let g:coc_disable_startup_warning = 1
" =============================================================================
" # PLUGINS
" =============================================================================

call plug#begin()

" Load plugins
" VIM enhancements
Plug 'editorconfig/editorconfig-vim'
Plug 'justinmk/vim-sneak'

" GUI enhancements
Plug 'itchyny/lightline.vim'
Plug 'machakann/vim-highlightedyank'
Plug 'andymass/vim-matchup'

" Fuzzy finder
" Plug 'airblade/vim-rooter'
" Plug 'junegunn/fzf', { 'dir': '~/.fzf', 'do': './install --all' }
" Plug 'junegunn/fzf.vim'

" Nerdy stuff
Plug 'preservim/nerdcommenter'
Plug 'preservim/nerdtree'
Plug 'Xuyuanp/nerdtree-git-plugin'
Plug 'PhilRunninger/nerdtree-visual-selection'
Plug 'tiagofumo/vim-nerdtree-syntax-highlight'
Plug 'ryanoasis/vim-devicons'

" git
Plug 'tpope/vim-fugitive'
" Git commit browser
" Plug 'junegunn/gv.vim'
Plug 'airblade/vim-gitgutter'

" Autocomplete framework
"Plug 'ncm2/ncm2'
"Plug 'ncm2/ncm2-bufword'
"Plug 'ncm2/ncm2-path'
"Plug 'roxma/nvim-yarp'
Plug 'hrsh7th/cmp-buffer'                            
Plug 'hrsh7th/cmp-nvim-lsp'
Plug 'hrsh7th/cmp-nvim-lsp-signature-help'
Plug 'hrsh7th/cmp-nvim-lua'
Plug 'hrsh7th/cmp-path'                              
Plug 'hrsh7th/cmp-vsnip'                             
Plug 'hrsh7th/nvim-cmp' 
Plug 'hrsh7th/vim-vsnip'  
Plug 'neovim/nvim-lspconfig'
" Plug 'neoclide/coc.nvim', {'branch': 'release'}

" base16 vim themes
Plug 'chriskempson/base16-vim'

" rust
Plug 'rust-lang/rust.vim'
" Plug 'ncm2/ncm2-racer'
" Plug 'racer-rust/vim-racer'
" Plug 'simrat39/rust-tools.nvim'
Plug 'kyazdani42/nvim-web-devicons'
Plug 'folke/trouble.nvim'
Plug 'kdarkhan/rust-tools.nvim'

" rust tree sitter
Plug 'nvim-treesitter/nvim-treesitter'

" Floating Term
Plug 'voldikss/vim-floaterm'

" move line/selection left/right up/down
Plug 'matze/vim-move'

" vim's conceal feature for additional visual eyecandy.
Plug 'khzaw/vim-conceal'

" Indent guides
Plug 'Yggdroot/indentLine'

" C plugins
Plug 'chazy/cscope_maps'

" Telescope
Plug 'nvim-lua/plenary.nvim'
Plug 'nvim-telescope/telescope.nvim'

call plug#end()

" system clipboard
set clipboard+=unnamedplus

" deal with colors
set t_Co=256

" screen does not (yet) support truecolor
set termguicolors

set background=dark
let base16colorspace=256
colorscheme base16-gruvbox-dark-hard
syntax on
hi Normal ctermbg=NONE

" Customize the highlight a bit.
" Make comments more prominent -- they are important.
call Base16hi("Comment", g:base16_gui09, "", g:base16_cterm09, "", "", "")
" Make it clearly visible which argument we're at.
call Base16hi("LspSignatureActiveParameter", g:base16_gui05, g:base16_gui03, g:base16_cterm05, g:base16_cterm03, "bold", "")
" Would be nice to customize the highlighting of warnings and the like to make
" them less glaring. But alas
" https://github.com/nvim-lua/lsp_extensions.nvim/issues/21
" call Base16hi("CocHintSign", g:base16_gui03, "", g:base16_cterm03, "", "", "")

" Lightline
let g:lightline = {
			\ 'colorscheme': 'one',
			\ 'active': {
			\   'left': [ [ 'mode', 'paste' ],
			\             [ 'readonly', 'filename', 'modified' ] ],
			\   'right': [ [ 'lineinfo' ],
			\              [ 'percent' ],
			\              [ 'fileencoding', 'filetype' ] ],
			\ },
			\ 'component_function': {
			\   'filename': 'LightlineFilename'
			\ },
			\ }
function! LightlineFilename()

endfunction

" from http://sheerun.net/2014/03/21/how-to-boost-your-vim-productivity/
if executable('ag')
	set grepprg=ag\ --nogroup\ --nocolor
endif
if executable('rg')
	set grepprg=rg\ --no-heading\ --vimgrep
	set grepformat=%f:%l:%c:%m
endif

lua << END
-- rust tools
local rt = require("rust-tools")

rt.setup({
server = {
	on_attach = function(_, bufnr)
	-- Hover actions
	vim.keymap.set("n", "<C-space>", rt.hover_actions.hover_actions, { buffer = bufnr })
	-- Code action groups
	vim.keymap.set("n", "<Leader>a", rt.code_action_group.code_action_group, { buffer = bufnr })
	end,
},
})

-- LSP Diagnostics Options Setup 
local sign = function(opts)
vim.fn.sign_define(opts.name, {
	texthl = opts.name,
	text = opts.text,
	numhl = ''
})
end

sign({name = 'DiagnosticSignError', text = ''})
sign({name = 'DiagnosticSignWarn', text = ''})
sign({name = 'DiagnosticSignHint', text = ''})
sign({name = 'DiagnosticSignInfo', text = ''})

vim.diagnostic.config({
virtual_text = false,
signs = true,
update_in_insert = true,
underline = true,
severity_sort = false,
float = {
	border = 'rounded',
	source = 'always',
	header = '',
	prefix = '',
},
})

vim.cmd([[
set signcolumn=yes
autocmd CursorHold * lua vim.diagnostic.open_float(nil, { focusable = false })
]])

--Set completeopt to have a better completion experience
-- :help completeopt
-- menuone: popup even when there's only one match
-- noinsert: Do not insert text until a selection is made
-- noselect: Do not select, force to select one from the menu
-- shortness: avoid showing extra messages when using completion
-- updatetime: set updatetime for CursorHold
vim.opt.completeopt = {'menuone', 'noselect', 'noinsert'}
vim.opt.shortmess = vim.opt.shortmess + { c = true}
vim.api.nvim_set_option('updatetime', 250) 

-- Fixed column for diagnostics to appear
-- Show autodiagnostic popup on cursor hover_range
-- Goto previous / next diagnostic warning / error 
-- Show inlay_hints more frequently 
vim.cmd([[
set signcolumn=yes
autocmd CursorHold * lua vim.diagnostic.open_float(nil, { focusable = false })
]])

-- Completion Plugin Setup
local cmp = require'cmp'
cmp.setup({
-- Enable LSP snippets
snippet = {
	expand = function(args)
	vim.fn["vsnip#anonymous"](args.body)
	end,
	},
	mapping = {
		['<C-p>'] = cmp.mapping.select_prev_item(),
		['<C-n>'] = cmp.mapping.select_next_item(),
		-- Add tab support
		['<S-Tab>'] = cmp.mapping.select_prev_item(),
		['<Tab>'] = cmp.mapping.select_next_item(),
		['<A-f>'] = cmp.mapping.scroll_docs(-4),
		['<C-f>'] = cmp.mapping.scroll_docs(4),
		['<C-Space>'] = cmp.mapping.complete(),
		['<C-e>'] = cmp.mapping.close(),
		['<CR>'] = cmp.mapping.confirm({
		behavior = cmp.ConfirmBehavior.Insert,
		select = true,
		})
		},
	-- Installed sources:
	sources = {
		{ name = 'path' },                              -- file paths
		{ name = 'nvim_lsp', keyword_length = 3 },      -- from language server
		{ name = 'nvim_lsp_signature_help'},            -- display function signatures with current parameter emphasized
		{ name = 'nvim_lua', keyword_length = 2},       -- complete neovim's Lua runtime API such vim.lsp.*
		{ name = 'buffer', keyword_length = 2 },        -- source current buffer
		{ name = 'vsnip', keyword_length = 2 },         -- nvim-cmp source for vim-vsnip 
		{ name = 'calc'},                               -- source for math calculation
	},
	window = {
		completion = cmp.config.window.bordered(),
		documentation = cmp.config.window.bordered(),
	},
	formatting = {
		fields = {'menu', 'abbr', 'kind'},
		format = function(entry, item)
		local menu_icon ={
		nvim_lsp = 'λ',
		vsnip = '⋗',
		buffer = 'Ω',
		path = '🖫',
		}
		item.menu = menu_icon[entry.source.name]
		return item
		end,
},
})

-- Treesitter Plugin Setup 
require('nvim-treesitter.configs').setup {
	ensure_installed = { "lua", "rust", "toml" },
	auto_install = true,
	highlight = {
		enable = true,
		additional_vim_regex_highlighting=false,
	},
	ident = { enable = true }, 
	rainbow = {
		enable = true,
		extended_mode = true,
		max_file_lines = nil,
	}
	}
-- Treesitter folding 
vim.wo.foldmethod = 'expr'
vim.wo.foldexpr = 'nvim_treesitter#foldexpr()'

-- Copilot
vim.g.copilot_assume_mapped = true

-- Telescope Plugin Setup
local builtin = require('telescope.builtin')
vim.keymap.set('n', '<leader>ff', builtin.find_files, {})
vim.keymap.set('n', '<leader>fg', builtin.live_grep, {})
vim.keymap.set('n', '<leader>fb', builtin.buffers, {})
vim.keymap.set('n', '<leader>fh', builtin.help_tags, {})
vim.keymap.set('n', '<leader>fc', builtin.commands, {})
vim.keymap.set('n', '<leader>fo', builtin.oldfiles, {})
vim.keymap.set('n', '<leader>ft', builtin.tags, {})

-- trouble Plugin Setup
require("trouble").setup {
-- your configuration comes here
-- or leave it empty to use the default settings
-- refer to the configuration section below
}

END

" Javascript
let javaScript_fold=0

" Java
let java_ignore_javadoc=1

" FloaTerm configuration
map <F1> :FloatermNew --wintype=normal --height=0.9 --width=0.9 --position=right --autoclose=2 --name=terminal<CR>
map <F2> :FloatermToggle terminal<CR>
map <F3> :FloatermHide<CR>

" Open hotkeys
map <C-p> :Files<CR>
nmap <leader>; :Buffers<CR>

" Quick-save
nmap <leader>w :w<CR>

" :X to save and exit
command! X wq

" Use Q as well to quit
command! -bang Q q<bang>

" Coc Explorer NVIM
" :nmap <space>e <Cmd>CocCommand explorer<CR>

" Always show the signcolumn, otherwise it would shift the text each time
" diagnostics appear/become resolved.
set signcolumn=yes
" disable signcolumn for tagbar, nerdtree, as thats useless
autocmd FileType tagbar,nerdtree setlocal signcolumn=no

""""""""""""""""""""""""""""""""
" NerdTree
""""""""""""""""""""""""""""""""
" Start NERDTree when Vim starts with a directory argument.
autocmd StdinReadPre * let s:std_in=1
autocmd VimEnter * if argc() == 1 && isdirectory(argv()[0]) && !exists('s:std_in') |
			\ execute 'NERDTree' argv()[0] | wincmd p | enew | execute 'cd '.argv()[0] | endif

" close vim if the only window left open is a NERDTree
nnoremap <silent> <leader>tt :NERDTreeToggle<CR>
nnoremap <silent> <leader>tf :NERDTreeFind<CR>

autocmd BufEnter * if (winnr("$") == 1 && exists("b:NERDTree") && b:NERDTree.isTabTree()) | q | endif

" If you are using vim-plug, you'll also need to add these lines to avoid crashes when calling vim-plug functions while the cursor is on the NERDTree window:
let g:plug_window = 'noautocmd vertical topleft new'

" show dot/hidden files
let NERDTreeShowHidden=1

""""""""""""""""""""""""""
" Rust
""""""""""""""""""""""""""
" run rustfmt on save
let g:rustfmt_autosave = 1
let g:rustfmt_emit_files = 1
let g:rustfmt_fail_silently = 0
let g:rust_clip_command = 'xclip -selection clipboard'

" Don't confirm .lvimrc
let g:localvimrc_ask = 0
" Better display for messages
set cmdheight=2
" You will have bad experience for diagnostic messages when it's default 4000.
set updatetime=300

" =============================================================================
" # Editor settings
" =============================================================================
filetype plugin indent on
set autoindent
set timeoutlen=300 " http://stackoverflow.com/questions/2158516/delay-before-o-opens-a-new-line
set encoding=utf-8
set scrolloff=2
set noshowmode
set hidden
set nowrap
set nojoinspaces
let g:sneak#s_next = 1
let g:vim_markdown_new_list_item_indent = 0
let g:vim_markdown_auto_insert_bullets = 0
let g:vim_markdown_frontmatter = 1
set printfont=:h10
set printencoding=utf-8
set printoptions=paper:letter
" Always draw sign column. Prevent buffer moving when adding/deleting sign.
set signcolumn=yes

" Settings needed for .lvimrc
set exrc
set secure

" Sane splits
set splitright
set splitbelow

" Permanent undo
set undodir=~/.vimdid
set undofile

" Decent wildmenu
set wildmenu
set wildmode=list:longest
set wildignore=.hg,.svn,*~,*.png,*.jpg,*.gif,*.settings,Thumbs.db,*.min.js,*.swp,publish/*,intermediate/*,*.o,*.hi,Zend,vendor

" tabs
set shiftwidth=4
set softtabstop=4
set tabstop=4
set noexpandtab
set smarttab

" Text width
set textwidth=80

" Wrapping options
set formatoptions=tc " wrap text and comments using textwidth
set formatoptions+=r " continue comments when pressing ENTER in I mode
set formatoptions+=q " enable formatting of comments with gq
set formatoptions+=n " detect lists for formatting
set formatoptions+=b " auto-wrap in insert mode, and do not wrap old long lines

" Proper search
set incsearch
set ignorecase
set smartcase
set gdefault

" Search results centered please
nnoremap <silent> n nzz
nnoremap <silent> N Nzz
nnoremap <silent> * *zz
nnoremap <silent> # #zz
nnoremap <silent> g* g*zz

" Very magic by default
nnoremap ? ?\v
" nnoremap / /\v
cnoremap %s/ %sm/

" Use tab for trigger completion with characters ahead and navigate.
" Use command ':verbose imap <tab>' to make sure tab is not mapped by other plugin.
"inoremap <silent><expr> <TAB>
			"\ coc#pum#visible() ? coc#pum#next(1) :
			"\ <SID>check_back_space() ? "\<TAB>" :
			"\ coc#refresh()
"inoremap <expr><S-TAB> pumvisible() ? "\<C-p>" : "\<C-h>"

" Use coc#pum#info() if you need to confirm completion,
" only when there selected complete item
"inoremap <silent><expr> <cr> coc#pum#visible() && coc#pum#info()['index'] != -1 ? coc#pum#confirm() : "\<C-g>u\<CR>"

"function! s:check_back_space() abort
"let col = col('.') - 1
"return !col || getline('.')[col - 1]  =~# '\s'
"endfunction

"" Use <c-space> to trigger completion.
"inoremap <silent><expr> <c-space> coc#refresh()

" =============================================================================
" # GUI settings
" =============================================================================
set guioptions-=T " Remove toolbar
set vb t_vb= " No more beeps
set backspace=2 " Backspace over newlines
set nofoldenable
set ttyfast
" https://github.com/vim/vim/issues/1735#issuecomment-383353563
set lazyredraw
set synmaxcol=500
set laststatus=2
set relativenumber " Relative line numbers
set number " Also show current absolute line
set diffopt+=iwhite " No whitespace in vimdiff
" Make diffing better: https://vimways.org/2018/the-power-of-diff/
set diffopt+=algorithm:patience
set diffopt+=indent-heuristic
set colorcolumn=80 " and give me a colored column
set showcmd " Show (partial) command in status line.
set mouse=a " Enable mouse usage (all modes) in terminals
set shortmess+=c " don't give |ins-completion-menu| messages.

" Show those damn hidden characters
" Verbose: set listchars=nbsp:¬,eol:¶,extends:»,precedes:«,trail:•
set listchars=nbsp:¬,extends:»,precedes:«,trail:•

" =============================================================================
" # Keyboard shortcuts
" =============================================================================
" ; as :
nnoremap ; :

" Ctrl+j and Ctrl+k as Esc
" Ctrl-j is a little awkward unfortunately:
" https://github.com/neovim/neovim/issues/5916
" So we also map Ctrl+k
nnoremap <C-j> <Esc>
inoremap <C-j> <Esc>
vnoremap <C-j> <Esc>
snoremap <C-j> <Esc>
xnoremap <C-j> <Esc>
cnoremap <C-j> <C-c>
onoremap <C-j> <Esc>
lnoremap <C-j> <Esc>
tnoremap <C-j> <Esc>

nnoremap <C-k> <Esc>
inoremap <C-k> <Esc>
vnoremap <C-k> <Esc>
snoremap <C-k> <Esc>
xnoremap <C-k> <Esc>
cnoremap <C-k> <C-c>
onoremap <C-k> <Esc>
lnoremap <C-k> <Esc>
tnoremap <C-k> <Esc>

" Ctrl+h to stop searching
vnoremap <C-h> :nohlsearch<cr>
nnoremap <C-h> :nohlsearch<cr>

" Suspend with Ctrl+f
inoremap <C-f> :sus<cr>
vnoremap <C-f> :sus<cr>
nnoremap <C-f> :sus<cr>

" Jump to start and end of line using the home row keys
map H ^
map L $

" Neat X clipboard integration
" ,p will paste clipboard into buffer
" ,c will copy entire buffer into clipboard
noremap <leader>p :read !xsel --clipboard --output<cr>
noremap <leader>c :w !xsel -ib<cr><cr>

" <leader>s for Rg search
"noremap <leader>s :Rg
"let g:fzf_layout = { 'down': '~20%' }
"command! -bang -nargs=* Rg
			"\ call fzf#vim#grep(
			"\   'rg --column --line-number --no-heading --color=always '.shellescape(<q-args>), 1,
			"\   <bang>0 ? fzf#vim#with_preview('up:60%')
			"\           : fzf#vim#with_preview('right:50%:hidden', '?'),
			"\   <bang>0)

"function! s:list_cmd()
	"let base = fnamemodify(expand('%'), ':h:.:S')
	"return base == '.' ? 'fdfind --type file --follow' : printf('fdfind --type file --follow | sort -u %s', shellescape(expand('%')))
"endfunction

"command! -bang -nargs=? -complete=dir Files
			"\ call fzf#vim#files(<q-args>, {'source': s:list_cmd(),
			"\                               'options': '--tiebreak=index'}, <bang>0)


" Open new file adjacent to current file
nnoremap <leader>o :e <C-R>=expand("%:p:h") . "/" <CR>

" Left and right can switch buffers
nnoremap <left> :bp<CR>
nnoremap <right> :bn<CR>

" Move by line
nnoremap j gj
nnoremap k gk

" <leader><leader> toggles between buffers
nnoremap <leader><leader> <c-^>

" <leader>, shows/hides hidden characters
nnoremap <leader>, :set invlist<cr>

" <leader>q shows stats
nnoremap <leader>q g<c-g>

noremap <leader>m ct_

" copy to system clipboard (requries xclip)
map <Leader>y "+y

" paste from system clipboard (requires xclip)
map <Leader>p "+p

" =============================================================================
" # Autocommands
" =============================================================================

" Prevent accidental writes to buffers that shouldn't be edited
autocmd BufRead *.orig set readonly
autocmd BufRead *.pacnew set readonly

" Jump to last edit position on opening file
if has("autocmd")
	" https://stackoverflow.com/questions/31449496/vim-ignore-specifc-file-in-autocommand
	au BufReadPost * if expand('%:p') !~# '\m/\.git/' && line("'\"") > 1 && line("'\"") <= line("$") | exe "normal! g'\"" | endif
endif

" Follow Rust code style rules
au Filetype rust source ~/.config/nvim/scripts/spacetab.vim
au Filetype rust set colorcolumn=100

" Help filetype detection
autocmd BufRead *.plot set filetype=gnuplot
autocmd BufRead *.md set filetype=markdown
autocmd BufRead *.lds set filetype=ld
autocmd BufRead *.tex set filetype=tex
autocmd BufRead *.trm set filetype=c
autocmd BufRead *.c set filetype=c
autocmd BufRead *.h set filetype=c
autocmd BufRead *.S set filetype=asm
autocmd BufRead *.s set filetype=asm
autocmd BufRead *.asm set filetype=asm
autocmd BufRead *.sh set filetype=sh
autocmd BufRead *.zsh set filetype=zsh
autocmd BufRead *.xlsx.axlsx set filetype=ruby

" Script plugins
autocmd Filetype html,xml,xsl,php source ~/.config/nvim/scripts/closetag.vim

source ~/.config/nvim/cscope_maps.vim

" =============================================================================
" # Footer
" =============================================================================
