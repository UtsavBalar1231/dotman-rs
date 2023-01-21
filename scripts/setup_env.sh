#!/usr/bin/env bash

if [ ! -f /etc/arch-release ]; then
	sudo apt update
	sudo apt install -y \
		git-core gnupg flex bc bison build-essential zip curl zlib1g-dev \
		gcc-multilib g++-multilib libncurses5 lib32ncurses5-dev libssl-dev libelf-dev \
		dwarves libxml2-utils xsltproc unzip rsync htop python3 python-is-python3 \
		ripgrep neofetch exa curl wget nodejs npm
else
	sudo pacman -Syu --noconfirm
	sudo pacman -S --noconfirm \
		git gnupg flex bison bc binutils bzip2 \
		ed gcc-libs grep gzip inetutils libarchive libelf libtool \
		linux-headers make pacman patch pkgconf sed sudo systemd \
		systemd-libs texinfo util-linux which xz exa curl wget \
		nodejs npm htop clang llvm llvm-libs
	# firefox-beta-bin slack-desktop touche libinput-gestures
	# whatsapp-nativefier
fi

# copy .clang-format file to home directory
cp -avx "$(pwd)"/.clang-format ~/.clang-format
