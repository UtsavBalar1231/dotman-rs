#!/usr/bin/env bash

# Set local timezone
export TZ="Asia/Kolkata"

# Setup build environment
bash $(pwd)/scripts/setup_git.sh
bash $(pwd)/scripts/setup_arch.sh

# Install necessary packages
sudo pacman -Sy
sudo pacman -S \
	fzf \
	bat \
	gotop \
	micro \
	neovim \
	ripgrep \
	python-neovim \
	tmux \
	zsh \
	-y

# Configure tmux
cp -avr $(pwd)/.tmux* ~/

# Install diff-so-fancy
if [ ! $(which diff-so-fancy) ]; then
	wget https://github.com/so-fancy/diff-so-fancy/releases/download/v1.4.3/diff-so-fancy
	chmod +x $(pwd)/diff-so-fancy
	sudo mv $(pwd)/diff-so-fancy /usr/local/bin/
fi

#
# Configure NeoVIM
#
# Installing vim-plug
if [ ! -f "${XDG_DATA_HOME:-$HOME/.local/share}"/nvim/site/autoload/plug.vim ]; then
	curl -fLo "${XDG_DATA_HOME:-$HOME/.local/share}"/nvim/site/autoload/plug.vim --create-dirs \
		https://raw.githubusercontent.com/junegunn/vim-plug/master/plug.vim
fi

# VIM configuration
cp -vr $(pwd)/nvim/ ~/.config/

# NVIM update and install plugins
echo -e "Run nvim comand:"
echo -e "nvim +PlugInstall +PlugUpdate +PlugClean +UpdateRemotePlugins"

# Install Oh My ZSH
if [ ! -e ${HOME}/.oh-my-zsh/.oh-my-zsh.sh ]; then
	bash -c "$(curl -fsSL https://raw.github.com/ohmyzsh/ohmyzsh/master/tools/install.sh)"
fi

sudo chsh $(which zsh)
cp -v $(pwd)/.zshrc ~/.zshrc

zsh $(pwd)/setup-zsh-dependencies.sh
