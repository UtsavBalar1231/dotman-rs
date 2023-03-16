local status_ok, dap = pcall(require, "dap")

if not status_ok then
	return
end

-- Python
dap.adapters.python = {
	type = "executable",
	command = "python",
	args = { "-m", "debugpy.adapter" },
}

dap.configurations.python = {
	{
		type = "python",
		request = "launch",
		name = "Launch file",
		program = "${file}",
	},
}
