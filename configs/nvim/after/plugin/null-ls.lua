local present, null_ls = pcall(require, "null-ls")

if not present then
	return
end

local b = null_ls.builtins

local sources = {
	b.completion.luasnip,

	-- Python
	b.formatting.black,
	b.diagnostics.flake8,

	-- Lua
	b.formatting.stylua,
	b.diagnostics.luacheck,

	-- bash
	b.formatting.shfmt,
	b.diagnostics.shellcheck,

	-- C/CPP
	b.formatting.clang_format,
	b.diagnostics.clang_check,

	-- Z-shell
	b.diagnostics.zsh,

	-- TO-DO comments
	b.diagnostics.todo_comments,

	-- Git
	b.code_actions.gitrebase,
	b.code_actions.gitsigns,
	b.diagnostics.gitlint,

	-- Markdown
	b.formatting.prettierd,
	b.completion.spell,
	b.diagnostics.markdownlint,

	-- css
	b.diagnostics.stylelint,

	-- Restructured Text
	b.diagnostics.rstcheck,
}

null_ls.setup({
	sources = sources,
})
