#!/usr/bin/env bash

if [ -f ~/.config/clang-format ]; then
	cp -f ~/.config/clang-format ~/.clang-format
fi

if [ -f ~/.config/gitconfig ]; then
	cp -f ~/.config/gitconfig ~/.gitconfig
fi

if [ -f ~/.config/zshrc ]; then
	cp -f ~/.config/zshrc ~/.zshrc
fi

debian_packages="gnupg flex bc bison build-essential zip curl zlib1g-dev gcc-multilib g++-multilib libncurses5 lib32ncurses-dev libssl-dev libelf-dev dwarves libxml2-utils xsltproc unzip rsync python3 python-is-python3 ripgrep exa curl wget nodejs npm kitty luarocks shellcheck"

arch_packages="gnupg flex bison bc binutils bzip2 ed gcc-libs grep gzip inetutils libarchive libelf libtool linux-headers make pacman patch pkgconf sed sudo systemd systemd-libs texinfo util-linux which xz exa curl wget nodejs npm clang llvm llvm-libs kitty luarocks shellcheck"

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
