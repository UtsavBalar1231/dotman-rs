local status_ok, comment = pcall(require, "Comment")

if not status_ok then
	vim.notify("Missing comment plugin", vim.log.levels.WARNING)
	return
end

comment.setup {
	pre_hook = require('ts_context_commentstring.integrations.comment_nvim').create_pre_hook(),
}
