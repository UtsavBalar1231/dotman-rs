#!/usr/bin/env bash

cp -avr "$(pwd)"/configs/* ~/.config/

if [ -f ~/.config/clang-format ]; then
	mv ~/.config/clang-format ~/.clang-format
fi

if [ -f ~/.config/gitconfig ]; then
	mv ~/.config/gitconfig ~/.gitconfig
fi

if [ -f ~/.config/zshrc ]; then
	mv ~/.config/zshrc ~/.zshrc
fi

if [ ! -f /etc/arch-release ]; then
	sudo apt update
	sudo apt install -y \
		gnupg flex bc bison build-essential zip curl zlib1g-dev \
		gcc-multilib g++-multilib libncurses5 lib32ncurses-dev libssl-dev libelf-dev \
		dwarves libxml2-utils xsltproc unzip rsync python3 python-is-python3 \
		ripgrep exa curl wget nodejs npm kitty luarocks shellcheck
else
	sudo pacman -Syu --noconfirm
	sudo pacman -S --noconfirm \
		gnupg flex bison bc binutils bzip2 \
		ed gcc-libs grep gzip inetutils libarchive libelf libtool \
		linux-headers make pacman patch pkgconf sed sudo systemd \
		systemd-libs texinfo util-linux which xz exa curl wget \
		nodejs npm clang llvm llvm-libs kitty luarocks shellcheck
fi
