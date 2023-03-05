local ensure_copilot = function()
    local fn = vim.fn
    local install_path = fn.stdpath("data") ..
                             "/site/pack/github/start/copilot.vim"
    -- Check if the copilot is already loaded
    if fn.empty(fn.glob(install_path)) > 0 then
        fn.system({
            "git", "clone", "--depth", "1",
            "https://github.com/github/copilot.vim", install_path
        })
    end
end

local copilot_check = ensure_copilot()
if copilot_check == 0 then vim.cmd([[packadd copilot.vim]]) end

vim.g.copilot_assume_mapped = true

-- Map <C-Space> to trigger copilot
vim.api.nvim_set_keymap("i", "<C-Space>", 'copilot#Accept("<CR>")',
                        {silent = true, expr = true})

vim.api.nvim_set_keymap("i", "<C-Right>", 'copilot#Accept("<CR>")',
                        {silent = true, expr = true})
