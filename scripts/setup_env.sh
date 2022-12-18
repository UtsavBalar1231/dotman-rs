#!/usr/bin/env bash

sudo apt update
sudo apt install git-core gnupg flex bc bison build-essential zip curl zlib1g-dev \
	gcc-multilib g++-multilib libncurses5 lib32ncurses5-dev libssl-dev libelf-dev \
	dwarves libxml2-utils xsltproc unzip rsync htop python3 python-is-python3 \
	ripgrep silversearcher-ag neofetch exa curl wget nodejs npm -y

sudo curl --create-dirs -L -o /usr/local/bin/repo -O -L https://storage.googleapis.com/git-repo-downloads/repo
sudo chmod a+rx /usr/local/bin/repo
