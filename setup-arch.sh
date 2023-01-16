#!/usr/bin/env bash

# Set local timezone
export TZ="Asia/Kolkata"

# Setup build environment
bash $(pwd)/scripts/setup_git.sh
bash $(pwd)/scripts/setup_env.sh

# Install necessary packages
sudo pacman -Sy --noconfirm
sudo pacman -S --noconfirm \
	fzf \
	bat \
	micro \
	neovim \
	ripgrep \
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
cp -vr $(pwd)/nvim/ ~/.config/

# Install Oh My ZSH
if [ ! -e ${HOME}/.oh-my-zsh/.oh-my-zsh.sh ]; then
	bash -c "$(curl -fsSL https://raw.github.com/ohmyzsh/ohmyzsh/master/tools/install.sh)"
fi

sudo chsh $(whoami)

zsh $(pwd)/setup-zsh-dependencies.sh
