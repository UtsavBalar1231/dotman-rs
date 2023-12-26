local status_ok, conform = pcall(require, "conform")

if not status_ok then
	vim.notify("Missing conform plugin", vim.log.levels.WARNING)
	return
end

conform.setup({
	-- Map of filetype to formatters
	formatters_by_ft = {
		bash = { "shfmt" },
		c = { "clang-format" },
		cpp = { "clang-format" },
		css = { "prettierd", "prettier" },
		javascript = { { "prettierd", "prettier" } },
		json = { "jq" },
		lua = { "stylua" },
		markdown = { "dprint" },
		python = { "black" },
		rust = { "rustfmt" },
		yaml = { "yamlfmt" },
		yml = { "yamlfmt" },
		["_"] = { "trim_whitespace" },
	},
	-- If this is set, Conform will run the formatter on save.
	-- It will pass the table to conform.format().
	-- This can also be a function that returns the table.
	format_on_save = {
		-- I recommend these options. See :help conform.format for details.
		lsp_fallback = false,
		timeout_ms = 500,
	},
	-- If this is set, Conform will run the formatter asynchronously after save.
	-- It will pass the table to conform.format().
	-- This can also be a function that returns the table.
	format_after_save = {
		lsp_fallback = false,
	},
	-- Set the log level. Use `:ConformInfo` to see the location of the log file.
	log_level = vim.log.levels.WARNING,
	-- Conform will notify you when a formatter errors
	notify_on_error = true,
})
