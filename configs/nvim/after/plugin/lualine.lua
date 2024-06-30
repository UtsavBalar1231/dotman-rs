local status_ok, lualine = pcall(require, "lualine")

if not status_ok then
	vim.notify("Missing lualine.nvim plugin", vim.log.levels.WARNING)
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
		warn = { fg = colors.orange },
		info = { fg = colors.blue },
	},
	update_in_insert = false,
	always_visible = false,
}

local diff = {
	"diff",
	colored = false,
	symbols = {
		modified = " ",
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
				return "[:o]"
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

local codeium = {
	"codeium",
	color = { fg = colors.green, gui = "bold" },
	fmt = function(_)
		local cstr = vim.fn["codeium#GetStatusString"]()
		if cstr == " ON" or cstr == " 0 " then
			return "codeium: on"
		elseif cstr == " * " then
			return " 󰔟 "
		else
			return cstr
		end
	end,
}

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
		lualine_x = {
			codeium,
		},
		lualine_y = {
			diagnostics,
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
