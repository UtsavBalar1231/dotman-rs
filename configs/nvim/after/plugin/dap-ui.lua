local status_ok, dapui = pcall(require, "dapui")

if not status_ok then
	return
end

dapui.setup()

require("nvim-dap-virtual-text").setup()
