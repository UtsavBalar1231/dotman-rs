local status_ok, null_ls = pcall(require, "null-ls")

if not status_ok then
	vim.notify("Missing none-ls plugin", vim.log.levels.WARNING)
	return
end

null_ls.setup({
	sources = {
		-- Code Actions
		null_ls.builtins.code_actions.gitsigns,

		-- Completions
		null_ls.builtins.completion.luasnip,
		null_ls.builtins.completion.spell,
		null_ls.builtins.completion.tags,

		-- Diagnostics
		null_ls.builtins.diagnostics.checkmake,
		null_ls.builtins.diagnostics.commitlint,
		null_ls.builtins.diagnostics.gitlint,
		null_ls.builtins.diagnostics.markdownlint,
		null_ls.builtins.diagnostics.pylint,
		null_ls.builtins.diagnostics.rstcheck,
		null_ls.builtins.diagnostics.stylelint,
		null_ls.builtins.diagnostics.todo_comments,
		null_ls.builtins.diagnostics.trail_space,
		null_ls.builtins.diagnostics.zsh,

		-- Formatting
		null_ls.builtins.formatting.blackd,
		null_ls.builtins.formatting.clang_format,
		null_ls.builtins.formatting.gofmt,
		null_ls.builtins.formatting.prettierd,
		null_ls.builtins.formatting.shfmt,
		null_ls.builtins.formatting.stylelint,
		null_ls.builtins.formatting.stylua,
		null_ls.builtins.formatting.stylua,
		null_ls.builtins.formatting.yamlfmt,

		-- Hover options
		null_ls.builtins.hover.dictionary,
		null_ls.builtins.hover.printenv,
	},
})
