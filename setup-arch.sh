#!/usr/bin/env bash

# Check if $DISPLAY is set
if [ -z "$DISPLAY" ]; then
	# Configure polybar: {{{
	sudo pacman --noconfirm -S polybar
	# }}}
fi

# NVIM configuration: {{{
if [ ! -d /root/.config ]; then
	sudo mkdir -p /root/.config
fi

if [ ! -d /root/.config/nvim ]; then
	sudo ln -s ~/.config/nvim/ /root/.config/nvim
fi

nvim --headless +PackerSync +qa

bash scripts/setup-nvim-lspservers.sh

# }}}

# Configure zsh: {{{
sudo chsh "$(whoami)" -s /bin/zsh
sudo chsh -s /bin/zsh

if [ ! -d /root/.config/zsh ]; then
	sudo ln -s ~/.config/zsh/ /root/.config/zsh
fi

echo "DO!:"
echo -e "\033[1;32msource ${HOME}/.zshrc\033[0m"
# }}}

zsh
