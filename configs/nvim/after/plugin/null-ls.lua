local present, null_ls = pcall(require, "null-ls")

if not present then
	return
end

local b = null_ls.builtins

local sources = {
	-- rust
	b.formatting.rustfmt,

	-- python
	b.formatting.black,
	b.diagnostics.flake8,

	-- lua
	b.formatting.stylua,
	b.diagnostics.luacheck,

	-- bash
	b.formatting.shfmt,
	b.diagnostics.shellcheck,

	-- c
	b.formatting.clang_format,
	b.diagnostics.clang_check,

	-- cpp
	b.formatting.clang_format,
	b.diagnostics.clang_check,

	-- zsh
	b.diagnostics.zsh,

	-- todo comments
	b.diagnostics.todo_comments,

	-- Git
	b.code_actions.gitrebase,
	b.code_actions.gitsigns,
	b.diagnostics.gitlint,
	b.diagnostics.commitlint,

	-- Markdown and Text
	b.completion.spell,
	b.diagnostics.codespell,
	b.diagnostics.textlint,
}

null_ls.setup({
	sources = sources,
})