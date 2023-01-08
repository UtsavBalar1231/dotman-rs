local wilder = require('wilder')

wilder.setup({modes = {':', '/', '?'}})

wilder.set_option('renderer', wilder.wildmenu_renderer({
    left = {' ', wilder.wildmenu_spinner(), ' '},
    right = {' ', wilder.wildmenu_index()}
}))

wilder.set_option('renderer',
                  wilder.popupmenu_renderer(
                      wilder.popupmenu_border_theme({
        highlights = {border = 'Normal'},
        border = 'rounded',
        left = {' ', wilder.popupmenu_devicons()},
        right = {' ', wilder.popupmenu_scrollbar()}
    })))

