#!/usr/bin/env bash

# Set local timezone
export TZ="Asia/Kolkata"

DEBIAN_VER=$(cat /etc/debian_version)

if [ -e "/etc/debian_version" ]; then
	echo -e "Debian ${DEBIAN_VER} detected"
	if (( $(echo "${DEBIAN_VER}" -gt 10 |bc -l) )); then
		exit 1
	else
		# Setup build environment
		bash "$(pwd)"/scripts/setup_git.sh
		bash "$(pwd)"/scripts/setup_env.sh
	fi
fi

# Install necessary packages
sudo apt-get update
sudo apt-get install \
	fd-find \
	fzf \
	neovim \
	tmux \
	thefuck \
	zsh \
	-y

# Configure bat
arch=$(dpkg --print-architecture)

function get_latest_release() {
	curl --silent "https://api.github.com/repos/$1/releases/latest" | # Get latest release from GitHub api
		grep '"tag_name":' |                                          # Get tag line
		sed -E 's/.*"([^"]+)".*/\1/'                                  # Pluck JSON value
	}

function bat_install() {
	VRELEASE=$(get_latest_release 'sharkdp/bat')
	RELEASE=$(echo "${VRELEASE}" | sed 's/v0/0/g')
	ARCHIVE=bat_${RELEASE}_${1}.deb
	wget https://github.com/sharkdp/bat/releases/download/"${VRELEASE}"/"${ARCHIVE}"
	sudo dpkg -i "${ARCHIVE}" && rm -f "${ARCHIVE}"
}

if [ -z "$(which bat)" ]; then
	bat_install "${arch}"
	$(which bat) --generate-config-file
	cp batconfig ~/.config/bat/config
fi

# Install diff-so-fancy
if [ -z "$(which diff-so-fancy)" ]; then
	wget https://github.com/so-fancy/diff-so-fancy/releases/download/v1.4.3/diff-so-fancy
	chmod +x "$(pwd)"/diff-so-fancy
	sudo mv "$(pwd)"/diff-so-fancy /usr/local/bin/
fi

# VIM configuration
sudo ln -s ~/.config/nvim /root/.config/nvim

sudo chsh -s "$(which zsh)" "$(whoami)"
sudo chsh -s "$(which zsh)" root

source "${HOME}"/.zshrc
