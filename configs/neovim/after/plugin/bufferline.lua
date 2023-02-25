local bufferline = require("bufferline")

bufferline.setup({
    options = {
        mode = "buffers",
        numbers = "ordinal",
        close_command = "bdelete! %d",
        right_mouse_command = "bdelete! %d",
        left_mouse_command = "buffer %d",
        middle_mouse_command = nil,
        color_icons = true,
        show_buffer_icons = true,
        seperator_style = "thin",
        offsets = {
            {
                filetype = "NvimTree",
                text = "File Explorer",
                highlight = "Directory",
                text_align = "right"
            }
        },
        diagnostics = "nvim_lsp",
        diagnostics_indicator = function(_, _, diagnostics_dict, _)
            local s = " "
            for e, n in pairs(diagnostics_dict) do
                local sym = e == "error" and " " or
                                (e == "warning" and " " or "")
                s = s .. n .. sym
            end
            return s
        end
    },
})
