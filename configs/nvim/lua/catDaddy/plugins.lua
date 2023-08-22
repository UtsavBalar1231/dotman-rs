local ensure_packer = function()
	local fn = vim.fn
	local install_path = fn.stdpath("data") .. "/site/pack/packer/start/packer.nvim"
	if fn.empty(fn.glob(install_path)) > 0 then
		fn.system({
			"git",
			"clone",
			"--depth",
			"1",
			"https://github.com/wbthomason/packer.nvim",
			install_path,
		})
		vim.cmd([[packadd packer.nvim]])
		return true
	end
	return false
end

local packer_bootstrap = ensure_packer()

return require("packer").startup(function(use)
	use("wbthomason/packer.nvim")

	-- Sexy gruvbox theme
	use("sainnhe/gruvbox-material")

	-- A fucking fast status line
	-- Requires nvim-web-devicons
	use("nvim-lualine/lualine.nvim")
	use("nvim-tree/nvim-web-devicons")

	-- match-up is a plugin that lets you highlight, navigate, and operate on sets of matching text.
	use({
		"andymass/vim-matchup",
		setup = function()
			vim.g.matchup_matchparen_offscreen = { method = "popup" }
		end,
	})

	-- Tree Sitter plugin
	use({ "nvim-treesitter/nvim-treesitter", run = ":TSUpdate" })
	use({ "nvim-treesitter/nvim-treesitter-textobjects" })
	use({
		"nvim-treesitter/nvim-treesitter-context",
		config = function()
			require("treesitter-context").setup({
				enable = true, -- Enable this plugin (Can be enabled/disabled later via commands)
				max_lines = 0, -- How many lines the window should span. Values <= 0 mean no limit.
				min_window_height = 0, -- Minimum editor window height to enable context. Values <= 0 mean no limit.
				line_numbers = true,
				multiline_threshold = 20, -- Maximum number of lines to collapse for a single context line
				trim_scope = "outer", -- Which context lines to discard if `max_lines` is exceeded. Choices: 'inner', 'outer'
				mode = "cursor", -- Line used to calculate context. Choices: 'cursor', 'topline'
				-- Separator between context and content. Should be a single character string, like '-'.
				-- When separator is set, the context will only show up when there are at least 2 lines above cursorline.
				separator = nil,
				zindex = 20, -- The Z-index of the context window
				on_attach = nil, -- (fun(buf: integer): boolean) return false to disable attaching
			})
		end,
	})

	-- Sphinx
	use({ "stsewd/sphinx.nvim", run = ":UpdateRemotePlugins" })

	--- Autocompletion and LSP {{{
	use("neovim/nvim-lspconfig")

	-- LSP Manager
	use("williamboman/mason.nvim")
	use("williamboman/mason-lspconfig.nvim") -- Autocompletion

	use("hrsh7th/nvim-cmp")
	use("hrsh7th/cmp-buffer")
	use("hrsh7th/cmp-path")
	use("hrsh7th/cmp-nvim-lsp")
	use("hrsh7th/cmp-nvim-lua")

	-- LSP Snippets
	use("saadparwaiz1/cmp_luasnip")
	use("L3MON4D3/LuaSnip")
	use("rafamadriz/friendly-snippets")

	-- LSP Formatting
	use({
		"creativenull/efmls-configs-nvim",
		requires = { "neovim/nvim-lspconfig" },
	})

	-- LSP UI
	use("onsails/lspkind-nvim")
	use({
		"NvChad/nvim-colorizer.lua",
		config = function()
			require("colorizer").setup()
		end,
	})

	-- LSP status
	use({
		"nvim-lua/lsp-status.nvim",
	})

	--- Autocompletion and LSP }}}

	-- Adds extra functionality over rust analyzer
	use("simrat39/rust-tools.nvim")
	use("rust-lang/rust.vim")
	use({
		"Saecki/crates.nvim",
		config = function()
			require("crates").setup()
		end,
	})

	-- Telescope
	use({
		"nvim-lua/plenary.nvim",
		"nvim-telescope/telescope.nvim",
		{
			"nvim-telescope/telescope-fzf-native.nvim",
			build = "make",
		},
		"nvim-telescope/telescope-file-browser.nvim",
	})

	-- GitHub
	use({
		"github/copilot.vim",
		"lewis6991/gitsigns.nvim",
	})

	-- Identline for better indent
	use("lukas-reineke/indent-blankline.nvim")

	-- Comments support
	use({ "numToStr/Comment.nvim", requires = "JoosepAlviste/nvim-ts-context-commentstring" })

	-- Buffer Line
	use({
		"willothy/nvim-cokeline",
		requires = "nvim-tree/nvim-web-devicons",
	})

	-- NVIM hop for better navigation
	use({"phaazon/hop.nvim",})

	-- Automatically set up configuration after cloning packer.nvim
	if packer_bootstrap then
		require("packer").sync()
	end
end)
