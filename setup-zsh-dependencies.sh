#!/usr/bin/env bash

# Install zsh-autosuggestions
if [ ! -d  ${ZSH_CUSTOM:-~/.oh-my-zsh/custom}/plugins/zsh-autosuggestions ]; then
	git clone --depth=1 https://github.com/zsh-users/zsh-autosuggestions ${ZSH_CUSTOM:-~/.oh-my-zsh/custom}/plugins/zsh-autosuggestions
fi

zsh

# Copy my custom zsh theme
cp -v $(pwd)/cunt-theme.zsh-theme ~/.oh-my-zsh/themes/

source ~/.zshrc
