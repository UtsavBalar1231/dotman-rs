#!/usr/bin/env bash

# Set local timezone
export TZ="Asia/Kolkata"

# Setup build environment
bash "$(pwd)"/scripts/setup_git.sh
bash "$(pwd)"/scripts/setup_env.sh

# Install necessary packages
sudo pacman -Sy --noconfirm
sudo pacman -S --noconfirm \
	fzf \
	bat \
	neovim \
	ripgrep \
	tmux \
	zsh \
	-y

# Check if $DISPLAY is set
if [ -z "$DISPLAY" ]; then
	# Configure polybar
	sudo pacman --noconfirm -S polybar
fi

"$(pwd)"/bin/sync-dotfiles-rs -F

# Install diff-so-fancy
if [ ! "$(which diff-so-fancy)" ]; then
	wget https://github.com/so-fancy/diff-so-fancy/releases/download/v1.4.3/diff-so-fancy
	chmod +x "$(pwd)"/diff-so-fancy
	sudo mv "$(pwd)"/diff-so-fancy /usr/local/bin/
fi

# VIM configuration
sudo ln -s ~/.config/nvim/ /root/.config/nvim

# Configure zsh
sudo chsh "$(whoami)" -s /bin/zsh
sudo chsh -s /bin/zsh

source "${HOME}"/.zshrc
