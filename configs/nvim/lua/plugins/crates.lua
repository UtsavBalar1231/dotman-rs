return {
	"saecki/crates.nvim",
	event = { "BufRead Cargo.toml Cargo.lock"},

	config = function()
		require("crates").setup()
	end,
}
