local status_ok, nvimlint = pcall(require, "lint")

if not status_ok then
	vim.notify("Missing nvim-lint plugin", vim.log.levels.WARNING)
	return
end

nvimlint.linters_by_ft = {
	javascript = { "eslint_d" },
	typescript = { "eslint_d" },
	javascriptreact = { "eslint_d" },
	typescriptreact = { "eslint_d" },
	markdown = { "markdownlint" },
	c = { "cpplint", "clandtidy" },
	cpp = { "cpplint", "clandtidy" },
	lua = { "luacheck" },
	rust = { "rustfmt" },
	bash = { "shellcheck" },
	zsh = { "shellcheck" },
	python = { "flake8" },
}
