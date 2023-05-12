#!/usr/bin/env bash

cp -avr "$(pwd)"/configs/* ~/.config/

mv ~/.config/clang-format ~/.clang-format
mv ~/.config/zshrc ~/.zshrc

if [ ! -f /etc/arch-release ]; then
	sudo apt update
	sudo apt install -y \
		git-core gnupg flex bc bison build-essential zip curl zlib1g-dev \
		gcc-multilib g++-multilib libncurses5 lib32ncurses5-dev libssl-dev libelf-dev \
		dwarves libxml2-utils xsltproc unzip rsync python3 python-is-python3 \
		ripgrep exa curl wget nodejs npm kitty luarocks shellcheck shfmt gcc-aarch64-linux-gnu
else
	sudo pacman -Syu --noconfirm
	sudo pacman -S --noconfirm \
		git gnupg flex bison bc binutils bzip2 \
		ed gcc-libs grep gzip inetutils libarchive libelf libtool \
		linux-headers make pacman patch pkgconf sed sudo systemd \
		systemd-libs texinfo util-linux which xz exa curl wget \
		nodejs npm clang llvm llvm-libs kitty luarocks shellcheck shfmt
fi
