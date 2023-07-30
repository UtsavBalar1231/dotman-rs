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
	use("kyazdani42/nvim-web-devicons")

	-- match-up is a plugin that lets you highlight, navigate, and operate on sets of matching text.
	use({
		"andymass/vim-matchup",
		setup = function()
			vim.g.matchup_matchparen_offscreen = { method = "popup" }
		end,
	})

	-- Tree Sitter plugin
	use({ "nvim-treesitter/nvim-treesitter", run = ":TSUpdate" })

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
		"jose-elias-alvarez/null-ls.nvim",
		config = function()
			require("colorizer").setup()
		end,
	})

	use({
		"jay-babu/mason-null-ls.nvim",
		dependencies = {
			"williamboman/mason.nvim",
			"jose-elias-alvarez/null-ls.nvim",
		},
	})

	-- LSP UI
	use("onsails/lspkind-nvim")
	use("NvChad/nvim-colorizer.lua")

	--- Autocompletion and LSP }}}

	-- Adds extra functionality over rust analyzer
	use("simrat39/rust-tools.nvim")
	use("rust-lang/rust.vim")
	use({
		"Saecki/crates.nvim",
		config = function()
			require("crates").setup({
				null_ls = {
					enabled = true,
					name = "crates.nvim",
				},
			})
		end,
	})

	-- Telescope
	use({
		"nvim-lua/plenary.nvim",
		"nvim-telescope/telescope.nvim",
		"nvim-telescope/telescope-fzf-native.nvim",
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
		requires = "kyazdani42/nvim-web-devicons",
	})

	-- Automatically set up configuration after cloning packer.nvim
	if packer_bootstrap then
		require("packer").sync()
	end
end)
