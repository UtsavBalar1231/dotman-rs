#!/usr/bin/env bash

# Check if $DISPLAY is set
if [ -z "$DISPLAY" ]; then
	# Configure polybar: {{{
	sudo pacman --noconfirm -S polybar
	# }}}
fi

# NVIM configuration: {{{
sudo ln -s ~/.config/nvim/ /root/.config/nvim

if ! command -v luarocks &>/dev/null; then
	sudo luarocks install luacheck
fi

bash scripts/setup-nvim-lspservers.sh

# }}}

# Configure zsh: {{{
sudo chsh "$(whoami)" -s /bin/zsh
sudo chsh -s /bin/zsh

echo "DO!:"
echo -e "\033[1;32msource ${HOME}/.zshrc\033[0m"
# }}}

zsh
