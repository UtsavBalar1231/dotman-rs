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

	-- Motion tool to jump to any location
	use({
		"justinmk/vim-sneak",
		config = function()
			vim.g["sneak#label"] = 1
			vim.g["sneak#s_next"] = 1
		end,
	})

	-- A fucking fast status line
	-- Requires nvim-web-devicons
	use({
		"nvim-lualine/lualine.nvim",
	})

	-- match-up is a plugin that lets you highlight, navigate, and operate on sets of matching text.
	use({
		"andymass/vim-matchup",
		setup = function()
			vim.g.matchup_matchparen_offscreen = { method = "popup" }
		end,
	})

	-- A File Explorer For Neovim Written In Lua
	use({
		"nvim-tree/nvim-tree.lua",
		requires = { "nvim-tree/nvim-web-devicons" },
		tag = "nightly",
	})

	-- Tree Sitter plugin
	use({ "nvim-treesitter/nvim-treesitter", run = ":TSUpdate" })

	-- A plugin for Neovim that provides a floating terminal
	use({
		"voldikss/vim-floaterm",
		config = function()
			vim.g.floaterm_width = 0.9
			vim.g.floaterm_height = 0.9
			vim.g.floaterm_keymap_toggle = "<F1>"
			vim.g.floaterm_keymap_new = "<F2>"
			vim.g.floaterm_keymap_prev = "<F3>"
			vim.g.floaterm_keymap_next = "<F4>"
			vim.g.floaterm_keymap_kill = "<F5>"
		end,
	})

	-- Better Wildmenu please
	use({
		"gelguy/wilder.nvim",
		requires = { "romgrk/fzy-lua-native" },
	})

	-- Git integration
	use("tpope/vim-fugitive")
	use("airblade/vim-gitgutter")

	-- Autocompletion and LSP
	use({
		"VonHeikemen/lsp-zero.nvim",
		requires = {
			-- LSP Support
			{ "neovim/nvim-lspconfig" },
			{ "williamboman/mason.nvim" },
			{ "williamboman/mason-lspconfig.nvim" },

			-- Autocompletion
			{ "hrsh7th/nvim-cmp" },
			{ "hrsh7th/cmp-buffer" },
			{ "hrsh7th/cmp-path" },
			{ "saadparwaiz1/cmp_luasnip" },
			{ "hrsh7th/cmp-nvim-lsp" },
			{ "hrsh7th/cmp-nvim-lua" },
			{ "hrsh7th/cmp-cmdline" },
			{ "hrsh7th/cmp-nvim-lsp-signature-help" },

			-- For vsnip users.
			{ "hrsh7th/cmp-vsnip" },
			{ "hrsh7th/vim-vsnip" },

			-- Snippets
			{ "L3MON4D3/LuaSnip" },
			-- Snippet Collection (Optional)
			{ "rafamadriz/friendly-snippets" },
		},
	})

	-- Visualize lsp progress
	use({
		"j-hui/fidget.nvim",
		config = function()
			require("fidget").setup()
		end,
	})

	-- Rust language support
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
		"nvim-lua/popup.nvim",
		"nvim-telescope/telescope-fzf-native.nvim",
	})

	-- Trouble diagnostics
	use("folke/trouble.nvim")

	-- Copilot github
	use("github/copilot.vim")

	-- Identline for better indent
	use("lukas-reineke/indent-blankline.nvim")

	-- Comments support
	use("numToStr/Comment.nvim")

	-- Formatter
	use("mhartington/formatter.nvim")

	-- Buffer Line nvim
	use({ "akinsho/bufferline.nvim", branch = "dev" })

	-- Automatically set up configuration after cloning packer.nvim
	if packer_bootstrap then
		require("packer").sync()
	end
end)
