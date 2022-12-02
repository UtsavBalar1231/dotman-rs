#!/usr/bin/env bash

# Set local timezone
export TZ="Asia/Kolkata"

# Install necessary packages
sudo apt-get update
sudo apt-get install \
	tmux \
	thefuck \
	neovim \
	fzf \
	fd-find \
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

# Configure bat
arch=$(dpkg --print-architecture)

function get_latest_release() {
    curl --silent "https://api.github.com/repos/$1/releases/latest" | # Get latest release from GitHub api
        grep '"tag_name":' |                                          # Get tag line
        sed -E 's/.*"([^"]+)".*/\1/'                                  # Pluck JSON value
}

function bat_install() {
    VRELEASE=$(get_latest_release 'sharkdp/bat')
    RELEASE=$(echo ${VRELEASE} | sed 's/v0/0/g')
    ARCHIVE=bat_${RELEASE}_${1}.deb
    wget https://github.com/sharkdp/bat/releases/download/${VRELEASE}/${ARCHIVE}
    sudo dpkg -i ${ARCHIVE}
    rm ${ARCHIVE}
}
bat_install ${arch}


$(which bat) --generate-config-file
cp batconfig ~/.config/bat/config

# Install gotop
git clone --depth=1 https://github.com/cjbassi/gotop /tmp/gotop
/tmp/gotop/scripts/download.sh
mv $(pwd)/gotop /usr/local/bin/

# Install micro editor
curl https://getmic.ro | bash
sudo install micro /usr/local/bin/micro

# Configure NeoVIM
#
# Installing vim-plug
curl -fLo "${XDG_DATA_HOME:-$HOME/.local/share}"/nvim/site/autoload/plug.vim --create-dirs \
       https://raw.githubusercontent.com/junegunn/vim-plug/master/plug.vim

# VIM configuration
cp -vr $(pwd)/nvim/ ~/.config/

# NVIM update and install plugins
nvim +PlugInstall +PlugUpdate +PlugClean +UpdateRemotePlugins
