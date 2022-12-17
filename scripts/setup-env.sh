#!/usr/bin/env bash

sudo apt-get update
sudo apt-get install \
	git-core gnupg flex bc bison build-essential zip curl zlib1g-dev \
	gcc-multilib g++-multilib libc6-dev-i386 libncurses5 lib32ncurses5-dev \
	libssl-dev libelf-dev dwarves libxml2-utils \
	xsltproc unzip rsync htop python python-is-python3 ripgrep \
	silversearcher-ag neofetch exa -y

sudo curl --create-dirs -L -o /usr/local/bin/repo -O -L https://storage.googleapis.com/git-repo-downloads/repo
sudo chmod a+rx /usr/local/bin/repo
