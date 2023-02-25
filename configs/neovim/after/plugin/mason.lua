require("mason").setup()

require("mason-lspconfig").setup({
    ensure_installed = {
        "bashls", "clangd", "pyright", "rust_analyzer", "lua_ls"
    },
    automatic_installation = true
})
