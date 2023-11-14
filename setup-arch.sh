#!/usr/bin/env bash

# Check if $DISPLAY is set
active_monitors=$(xrandr | grep -c " connected")
if [ -n "$active_monitors" ]; then
	# Configure polybar: {{{
	sudo pacman --noconfirm -S polybar
	# }}}
fi

# NVIM configuration: {{{
if [ ! -d /root/.config ]; then
	sudo mkdir -p /root/.config
fi

if [ ! -d /root/.config/nvim ]; then
	sudo cp -afr ~/.config/nvim/ /root/.config/nvim
fi

nvim --headless +PackerSync +qa
# }}}

# Configure zsh: {{{
sudo chsh "$(whoami)" -s "$(which zsh)"
sudo chsh -s "$(which zsh)"

if [ ! -d /root/.config/zsh ]; then
	sudo cp -afr ~/.config/zsh/ /root/.config/zsh
fi

# shellcheck disable=SC1090
source ~/.zshrc
# }}}

exec $(which zsh)
