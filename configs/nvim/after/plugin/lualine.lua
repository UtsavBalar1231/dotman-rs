local status_ok, lualine = pcall(require, "lualine")
if not status_ok then
	return
end

local colors = {
	bg = "#1d2021",
	fg = "#ebdbb2",
	aqua = "#8ec07c",
	darkblue = "#076678",
	orange = "#fe8019",
	violet = "#b16286",
	purple = "#d3869b",
	red = "#fb4934",
	green = "#b8bb26",
	blue = "#83a598",
	yellow = "#fe8019",
	grey = "#a89984",
}

local diagnostics = {
	"diagnostics",
	sources = { "nvim_diagnostic", "coc" },
	sections = { "error", "warn", "info" },
	symbols = { error = " ", warn = " ", info = " " },
	diagnostics_color = {
		error = { fg = colors.red },
		warn = { fg = colors.yellow },
		info = { fg = colors.blue },
	},
	update_in_insert = false,
	always_visible = false,
}

local diff = {
	"diff",
	colored = false,
	symbols = {
		modified = "柳",
		added = " ",
		removed = " ",
	},
}

local filetype = {
	"filetype",
	icons_enabled = false,
	icon = nil,
	fmt = function(str)
		if not (str == nil or str == "") then
			if str == "markdown" then
				return ".md"
			else
				return "." .. str
			end
		else
			return 'Open a file with ":e"'
		end
	end,
}

local branch = {
	"branch",
	icons_enabled = false,
	fmt = function(str)
		if str == nil or str == "" then
			local mode = vim.fn.mode()
			if mode == "n" then
				return ":)"
			elseif mode == "i" then
				return ":O"
			elseif mode == "v" then
				return ":v"
			elseif mode == "V" then
				return ":V"
			elseif mode == "" then
				return "^ V ^"
			elseif mode == "R" then
				return "-_-"
			elseif mode == "t" then
				return "'_'"
			else
				return "(╯°□°)╯"
			end
		else
			return "git:" .. str
		end
	end,
}

local function starts_with(str, start)
	return str:sub(1, #start) == start
end

local filename = {
	"filename",
	color = { fg = colors.bg, bg = colors.grey, gui = "bold" },
	file_status = true,
	symbols = {
		readonly = "",
		modified = "",
		unreadable = "",
		new = "",
	},
	path = 3,
}

local filesize = {
	"filesize",
	cond = function()
		return vim.fn.empty(vim.fn.expand("%:t")) ~= 1
	end,
	fmt = function(str)
		return string.format("%sb", str)
	end,
}

local progress = {
	"progress",
	fmt = function(str)
		if not (str == "Top" or str == "Bot") then
			return str
		else
			if str == "Bot" then
				return "EOF"
			elseif str == "Top" then
				return "TOF"
			end
		end
	end,
}

local encoding = {
	"encoding",
	color = { gui = "bold" },
	fmt = function(str)
		return string.upper(str)
	end,
}

local lsp_name = function()
	local msg = "No Active Lsp"
	local buf_ft = vim.api.nvim_buf_get_option(0, "filetype")
	local clients = vim.lsp.get_active_clients()
	if next(clients) == nil then
		return msg
	end
	for _, client in ipairs(clients) do
		local filetypes = client.config.filetypes
		if filetypes and vim.fn.index(filetypes, buf_ft) ~= -1 then
			return client.name
		end
	end
	return msg
end

lualine.setup({
	options = {
		icons_enabled = true,
		theme = "auto",
		component_separators = { left = "", right = "" },
		section_separators = { left = "", right = "" },
		disabled_filetypes = { "alpha", "dashboard", "NvimTree", "Outline" },
		always_divide_middle = true,
	},
	sections = {
		lualine_a = { branch, diff },
		lualine_b = { "mode" },
		lualine_c = { filename, filesize, filetype, progress },
		lualine_x = { diagnostics },
		lualine_y = {
			{
				lsp_name,
				icon = " LSP:",
			},
			encoding,
		},
		lualine_z = { "location" },
	},
	inactive_sections = {
		lualine_a = {},
		lualine_b = {},
		lualine_c = {},
		lualine_x = {},
		lualine_y = {},
		lualine_z = {},
	},
	tabline = {},
	extensions = {},
})
