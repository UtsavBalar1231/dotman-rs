#!/usr/bin/env zsh

# Install zsh-autosuggestions
if [ ! -d  ${ZSH_CUSTOM:-~/.oh-my-zsh/custom}/plugins/zsh-autosuggestions ]; then
	git clone --depth=1 https://github.com/zsh-users/zsh-autosuggestions ${ZSH_CUSTOM:-~/.oh-my-zsh/custom}/plugins/zsh-autosuggestions
fi

if [ ! -d ${ZSH_CUSTOM:-~/.oh-my-zsh/custom}/plugins/F-Sy-H ]; then
	git clone https://github.com/z-shell/F-Sy-H.git ${ZSH_CUSTOM:-~/.oh-my-zsh/custom}/plugins/F-Sy-H
fi

# Copy my custom zsh theme
cp -v $(pwd)/cunt-theme.zsh-theme ~/.oh-my-zsh/themes/

source ~/.zshrc
