return {
	"hrsh7th/nvim-cmp",
	event = "InsertEnter",

	dependencies = {
		"hrsh7th/cmp-buffer",
		"hrsh7th/cmp-path",
		"hrsh7th/cmp-nvim-lsp",
		"hrsh7th/cmp-nvim-lua",
		"hrsh7th/cmp-nvim-lsp-signature-help",

		"saadparwaiz1/cmp_luasnip",
		"L3MON4D3/LuaSnip",
		"rafamadriz/friendly-snippets",
		"onsails/lspkind.nvim",

		"Exafunction/codeium.vim",
		"Saecki/crates.nvim",
		"brenoprata10/nvim-highlight-colors",
	},

	config = function()
		local cmp = require("cmp")
		local luasnip = require("luasnip")
		local lspkind = require("lspkind")

		local check_backspace = function()
			local col = vim.fn.col(".") - 1
			return col == 0 or vim.fn.getline("."):sub(col, col):match("%s")
		end

		-- vscode format
		require("luasnip.loaders.from_vscode").lazy_load({ exclude = vim.g.vscode_snippets_exclude or {} })
		require("luasnip.loaders.from_vscode").lazy_load({ paths = vim.g.vscode_snippets_path or "" })

		-- snipmate format
		require("luasnip.loaders.from_snipmate").load()
		require("luasnip.loaders.from_snipmate").lazy_load({ paths = vim.g.snipmate_snippets_path or "" })

		-- lua format
		require("luasnip.loaders.from_lua").load()
		require("luasnip.loaders.from_lua").lazy_load({ paths = vim.g.lua_snippets_path or "" })

		vim.api.nvim_create_autocmd("InsertLeave", {
			callback = function()
				if
					require("luasnip").session.current_nodes[vim.api.nvim_get_current_buf()]
					and not require("luasnip").session.jump_active
				then
					require("luasnip").unlink_current()
				end
			end,
		})

		require("luasnip.loaders.from_vscode").lazy_load()

		vim.api.nvim_set_hl(0, "CmpGhostText", { link = "Comment", default = true })

		cmp.setup({
			enabled = function()
				-- disable completion in comments
				local context = require("cmp.config.context")
				-- keep command mode completion enabled when cursor is in a comment
				local mode = vim.api.nvim_get_mode()
				---@diagnostic disable-next-line: undefined-field
				if string.sub(mode.mode, 1, 1) == "c" then
					return true
				else
					return not context.in_treesitter_capture("comment") and not context.in_syntax_group("Comment")
				end
			end,
			completion = {
				completeopt = "menu,menuone,preview,noselect",
				keyword_length = 1,
			},
			snippet = { -- configure how nvim-cmp interacts with snippet engine
				expand = function(args)
					luasnip.lsp_expand(args.body)
				end,
			},
			formatting = {
				fields = { "kind", "abbr", "menu" },
				format = function(entry, vim_item)
					local color_item = require("nvim-highlight-colors").format(entry, { kind = vim_item.kind })
					local kind = lspkind.cmp_format({ mode = "symbol_text", maxwidth = 50 })(entry, vim_item)
					local strings = vim.split(kind.kind, "%s", { trimempty = true })
					if color_item.abbr_hl_group then
						vim_item.kind_hl_group = color_item.abbr_hl_group
						vim_item.kind = color_item.abbr
					else
						kind.kind = " " .. (strings[1] or "") .. " "
					end

					kind.menu = "    (" .. (strings[2] or "") .. ")"

					return kind
				end,
			},

			preselect = cmp.PreselectMode.None,
			experimental = {
				ghost_text = false,
			},
			duplicates = {
				codeium = 1,
				nvim_lsp = 1,
				luasnip = 1,
				buffer = 1,
				path = 1,
			},
			confirm_opts = {
				behavior = cmp.ConfirmBehavior.Replace,
				select = false,
			},
			window = {
				highlight_hovered_item = true,
				highlight_selected_item = true,
				completion = {
					border = "rounded",
					winhighlight = "Normal:Normal,FloatBorder:CmpBorder,CursorLine:Visual,Search:None",
					scrollbar = false,
				},
				documentation = {
					border = "rounded",
					winhighlight = "Normal:Normal,FloatBorder:CmpBorder,CursorLine:Visual,Search:None",
					scrollbar = false,
				},
			},
			mapping = cmp.mapping.preset.insert({
				-- snippet movement with luasnip
				["<Tab>"] = cmp.mapping(function(fallback)
					if cmp.visible() then
						cmp.select_next_item()
					elseif luasnip.expand_or_jumpable() then
						luasnip.expand_or_jump()
					elseif check_backspace() then
						fallback()
					else
						fallback()
					end
				end, { "i", "s" }),
				["<S-Tab>"] = cmp.mapping(function(fallback)
					if cmp.visible() then
						cmp.select_prev_item()
					elseif luasnip.jumpable(-1) then
						luasnip.jump(-1)
					else
						fallback()
					end
				end, { "i", "s" }),

				["<CR>"] = cmp.mapping({
					i = cmp.mapping.confirm({ behavior = cmp.ConfirmBehavior.Replace, select = false }),
					c = function(fallback)
						if cmp.visible() then
							cmp.confirm({ behavior = cmp.ConfirmBehavior.Replace, select = false })
						else
							fallback()
						end
					end,
				}, { "i", "c" }),
				["<C-Up>"] = cmp.mapping(cmp.mapping.scroll_docs(-4), { "i", "c" }),
				["<C-Down>"] = cmp.mapping(cmp.mapping.scroll_docs(4), { "i", "c" }),
			}),

			-- Installed sources
			sources = {
				{ name = "codeium", priority = 1000 },
				{ name = "nvim_lsp", priority = 850 },
				{ name = "luasnip", priority = 700 },
				{ name = "buffer", priority = 650 },
				{ name = "nvim_lua", priority = 600 },
				{ name = "path" },
				{ name = "crates" },
				{ name = "nvim_lsp_signature_help" },
			},

			cmp.setup.filetype({ "gitcommit" }, {
				sources = {
					{ name = "cmp_git" },
					{ name = "buffer" },
				},
			}),

			cmp.setup.filetype({ "toml", "rs" }, {
				sources = {
					{ name = "luasnip" },
					{ name = "crates" },
				},
			}),
		})

		vim.api.nvim_set_hl(0, "CmpBorder", { bg = "None", fg = "#5D5855" })
	end,
}
