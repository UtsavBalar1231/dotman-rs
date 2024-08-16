local status_ok, comment = pcall(require, "Comment")

if not status_ok then
	vim.notify("Missing Comment plugin", vim.log.levels.WARNING)
	return
end

local commentstring_avail, commentstring = pcall(require, "ts_context_commentstring.integrations.comment_nvim")

if commentstring_avail then
	comment.setup({
		pre_hook = commentstring.create_pre_hook()
	})
end
