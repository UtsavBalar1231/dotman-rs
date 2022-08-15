#!/usr/bin/env bash

# Set local timezone
export TZ="Asia/Kolkata"

# Install necessary packages
sudo apt-get update
sudo apt-get install \
	tmux \
	batcat \
	zsh \
	-y

# Install Oh My ZSH
sh -c "$(curl -fsSL https://raw.github.com/ohmyzsh/ohmyzsh/master/tools/install.sh)"
sudo chsh $(which zsh)
cp $(pwd)/.zshrc ~/.zshrc
sudo cp -r $(pwd)/.oh-my-zsh/* ~/.oh-my-zsh/

# Configure tmux
cp $(pwd)/.tmux.conf ~/

# Copy local binaries
sudo cp $(pwd)/bin/* /usr/local/bin

# Setup build environment
bash ./scripts/setup-git.sh
bash ./scripts/setup-android-environment.sh

source ~/.zshrc

# Configure bat
bat --generate-config-file
cp batconfig ~/.config/bat/config
