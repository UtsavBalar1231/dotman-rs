local status_ok, kitty = pcall(require, "kitty-scrollback")

if not status_ok then
	vim.notify("Missing Kitty scrollback plugin", vim.log.levels.WARNING)
	return
end

---@diagnostic disable-next-line: inject-field
vim.g.maplocalleader = ","

vim.keymap.set({ "n" }, "<C-e>", "5<C-e>", {})
vim.keymap.set({ "n" }, "<C-y>", "5<C-y>", {})

vim.opt.runtimepath:append(vim.fn.stdpath("data") .. "/site/pack/packer/opt/kitty-scrollback.nvim") -- packer
kitty.setup({})
