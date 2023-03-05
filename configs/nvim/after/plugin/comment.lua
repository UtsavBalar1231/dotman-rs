local status_ok, comment = pcall(require, "Comment")

if not status_ok then
	return
end

comment.setup({
    -- Add a space b/w comment and the line
    padding = true,
    -- Whether the cursor should stay at its position
    sticky = true,
    -- LHS of toggle mappings in NORMAL mode
    toggler = {
        -- Line-comment toggle keymap
        line = "<leader>l",
        -- Block-comment toggle keymap
        block = "<leader>b"
    },

    -- LHS of operator-pending mappings in NORMAL/VISUAL mode
    operator = {
        -- Line-comment keymap
        line = "gc",
        -- Block-comment keymap
        block = "gb"
    },
})
