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

	-- Themes
	-- use({ "catppuccin/nvim" })
	-- use("rebelot/kanagawa.nvim")
	use("ellisonleao/gruvbox.nvim")

	-- A fucking fast status line
	-- Requires nvim-web-devicons
	use({
		"nvim-lualine/lualine.nvim",
		requires = {
			"nvim-tree/nvim-web-devicons",
		},
	})

	-- LSP progress
	use({
		"j-hui/fidget.nvim",
		config = function()
			require("fidget").setup()
		end,
	})

	-- Tree Sitter plugin
	use({ "nvim-treesitter/nvim-treesitter", run = ":TSUpdate" })
	use({ "nvim-treesitter/nvim-treesitter-textobjects" })
	use({ "nvim-treesitter/nvim-treesitter-context" })

	-- Sphinx
	-- use({ "stsewd/sphinx.nvim", run = ":UpdateRemotePlugins" })

	--- Autocompletion and LSP {{{
	use("neovim/nvim-lspconfig")

	-- LSP Manager
	use("hrsh7th/nvim-cmp")
	use("hrsh7th/cmp-buffer")
	use("hrsh7th/cmp-path")
	use("hrsh7th/cmp-nvim-lsp")
	use("hrsh7th/cmp-nvim-lua")

	-- LSP Snippets
	use("saadparwaiz1/cmp_luasnip")
	use("L3MON4D3/LuaSnip")
	use("rafamadriz/friendly-snippets")

	-- LSP Linting, Diagnostics, Code-Completions and Formatting
	use({
		"nvimtools/none-ls.nvim",
		requires = {
			"nvim-lua/plenary.nvim",
		},
	})

	-- LSP Manager
	use("williamboman/mason.nvim")
	use("williamboman/mason-lspconfig.nvim")
	use({
		"jay-babu/mason-null-ls.nvim",

		requires = {
			"williamboman/mason.nvim",
			"nvimtools/none-ls.nvim",
		},
	})

	-- LSP UI
	use("brenoprata10/nvim-highlight-colors")

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
		"nvim-telescope/telescope-frecency.nvim",
		"nvim-telescope/telescope-file-browser.nvim",
	})

	-- GitHub
	use({
		"lewis6991/gitsigns.nvim",
	})

	-- Codeium
	use({
		"Exafunction/codeium.vim",
	})

	use({ "junegunn/fzf.vim" })

	-- Identline for better indent
	use("lukas-reineke/indent-blankline.nvim")

	-- Comments support
	use({ "JoosepAlviste/nvim-ts-context-commentstring" })
	use {
		'numToStr/Comment.nvim',
		config = function()
			require('Comment').setup()
		end
	}

	-- Kitty Scrollback
	use({
		"mikesmithgh/kitty-scrollback.nvim",
		disable = false,
		opt = true,
		cmd = { "KittyScrollbackGenerateKittens", "KittyScrollbackCheckHealth" },
		config = function()
			require("kitty-scrollback").setup()
		end,
	})

	-- Buffer Line
	use({
		"willothy/nvim-cokeline",
		requires = "nvim-tree/nvim-web-devicons",
	})

	-- NVIM hop for better navigation
	use({ "phaazon/hop.nvim" })

	-- Aerial code navigation
	use({
		"stevearc/aerial.nvim",
		config = function()
			require("aerial").setup()
		end,
	})

	-- Automatically set up configuration after cloning packer.nvim
	if packer_bootstrap then
		require("packer").sync()
	end
end)
