#!/usr/bin/env bash

debian_packages="
bc
bison
build-essential
bzip2
clang
curl
dwarves
flex
fzf
g++-multilib
gcc-multilib
gnupg
kitty
lib32ncurses-dev
libelf-dev
libncurses5-dev
libssl-dev
libxml2-utils
luarocks
p7zip-full
python-is-python3
python3
rsync
shellcheck
stow
svls
tmux
unzip
wget
xclip
xsel
xsltproc
zip
zlib1g-dev
zsh
"

if [ ! -f /etc/arch-release ]; then
	sudo apt update -y

	IFS=$'\n' read -rd ' ' -a packages <<<"$debian_packages"
	for package in "${packages[@]}"; do
		if ! dpkg -s "$package" >/dev/null 2>&1; then
			sudo apt install -y "$package"
		else
			echo "$package is already installed"
		fi
	done
fi
