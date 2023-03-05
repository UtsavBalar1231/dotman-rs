
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
	use("hrsh7th/cmp-cmdline")

	-- LSP Snippets
	use("hrsh7th/cmp-vsnip")
	use("hrsh7th/vim-vsnip")
	use("rafamadriz/friendly-snippets")

	-- LSP Formatting
	use("jose-elias-alvarez/null-ls.nvim")

	-- LSP UI
	use("onsails/lspkind-nvim")
	use({
		"glepnir/lspsaga.nvim",
		branch = "main",
	})

	--- Autocompletion and LSP }}}

	-- Visualize lsp progress
	use({
		"j-hui/fidget.nvim",
		config = function()
			require("fidget").setup()
		end,
	})

	-- Adds extra functionality over rust analyzer
	use("simrat39/rust-tools.nvim")
	use("rust-lang/rust.vim")

	-- Markdown support
	-- install without yarn or npm
	use({
		"iamcco/markdown-preview.nvim",
		run = function()
			vim.fn["mkdp#util#install"]()
		end,
	})

	-- Telescope
	use({
		"nvim-lua/plenary.nvim",
		"nvim-telescope/telescope.nvim",
		"nvim-telescope/telescope-fzf-native.nvim",
		"nvim-telescope/telescope-file-browser.nvim",
		requires = { "nvim-lua/popup.nvim" },
	})

	-- Fithub
	use({
		"github/copilot.vim",
		"lewis6991/gitsigns.nvim",
		"dinhhuy258/git.nvim",
	})

	-- Identline for better indent
	use("lukas-reineke/indent-blankline.nvim")

	-- Comments support
	use({ "numToStr/Comment.nvim", requires = {
		"JoosepAlviste/nvim-ts-context-commentstring",
	} })

	-- Buffer Line nvim
	use({ "akinsho/bufferline.nvim", branch = "dev" })

	-- Automatically set up configuration after cloning packer.nvim
	if packer_bootstrap then
		require("packer").sync()
	end
end)
