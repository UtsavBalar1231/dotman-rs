return {
	"mrcjkb/rustaceanvim",
	lazy = false,
	ft = { "rust" },
	config = function()
		vim.g.rustaceanvim = {
			server = {
				on_attach = function(_, _)
					if vim.lsp.inlay_hint then
						vim.lsp.inlay_hint.enable(true, { 0 })
					end
					local map = vim.keymap.set
					map("n", "<leader>a", "<cmd>RustLsp codeAction<CR>", { desc = "Rust: Code actions" })
					map("n", "K", "<cmd>RustLsp hover actions<CR>", { desc = "Rust: Hover actions" })

					map("n", "co", "<cmd>RustLsp openCargo<CR>", { desc = "Rust: Go to cargo.toml" })
					map(
						"n",
						"<C-w>co",
						"<C-w>v<cmd>RustLsp openCargo<CR>",
						{ desc = "Rust: Go to cargo.toml (in new window)" }
					)
					map("n", "<leader>ee", "<cmd>RustLsp explainError<CR>", { desc = "Rust: Explain error" })
					map("n", "<leader>mj", "<cmd>RustLsp moveItem down<CR>", { desc = "Rust: Move item down" })
					map("n", "<leader>mk", "<cmd>RustLsp moveItem up<CR>", { desc = "Rust: Move item up" })
					map("n", "<leader>dd", "<cmd>RustLsp debug<CR>", { desc = "Rust: Debug item under cursor" })
					map("n", "<leader>D", "<cmd>RustLsp debuggables last<CR>", { desc = "Rust: Debug" })
					map("n", "<leader>dD", "<cmd>RustLsp renderDiagnostic<CR>", { desc = "Rust: Render diagnostics" })
					map("n", "<leader>d", "<cmd>RustLsp relatedDiagnostic<CR>", { desc = "Rust: Show diagnostics" })
					map("n", "<leader>mm", "<cmd>RustLsp expandMacro<CR>", { desc = "Rust: Expand macro" })
					map("n", "<leader>r", "<cmd>RustLsp run<CR>", { desc = "Rust: Run" })
					map("n", "<leader>R", "<cmd>RustLsp! run<CR>", { desc = "Rust: Rerun latest run" })

					-- openDocs
					map("n", "<leader>od", "<cmd>RustLsp openDocs<CR>", { desc = "Rust: Open docs" })
				end,

				settings = {
					["rust-analyzer"] = {
						diagnostics = {
							enable = true,
						},
						cargo = {
							allFeatures = true,
							buildScripts = {
								enable = true,
							},
						},
						procMacro = {
							enable = true,
						},
						add_return_type = {
							enable = true,
						},
					},
				},
			},
		}
	end,
}
