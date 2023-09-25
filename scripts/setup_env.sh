#!/usr/bin/env bash

debian_packages="bc bison build-essential bzip2 clang curl dwarves flex fzf gcc-multilib g++-multilib gnupg kitty lib32ncurses-dev libelf-dev libncurses5-dev libssl-dev libxml2-utils luarocks p7zip-full python3 python-is-python3 rsync shellcheck tmux unzip wget xclip xsel xsltproc zip zlib1g-dev zsh svls"

arch_packages="bat bc binutils bison btop bzip2 clang curl ed eza fd flex fzf gcc-libs gnupg gzip kitty libarchive libelf libtool llvm llvm-libs luarocks make neovim ncurses nodejs npm patch pkgconf ripgrep sed shellcheck sudo systemd systemd-libs texinfo tmux util-linux wget which xclip xsel xz zsh"

if [ ! -f /etc/arch-release ]; then
	sudo apt update -y

	IFS=' ' read -r -a packages <<< "$debian_packages"
	for package in "${packages[@]}"; do
		if ! dpkg -s "$package" >/dev/null 2>&1; then
			sudo apt install -y "$package"
		else
			echo "$package is already installed"
		fi
	done
else
	sudo pacman -Syu --noconfirm

	IFS=' ' read -r -a packages <<< "$arch_packages"
	for package in "${packages[@]}"; do
		if ! pacman -Qi "$package" >/dev/null 2>&1; then
			sudo pacman -S --noconfirm "$package"
		else
			echo "$package is already installed"
		fi
	done
fi
