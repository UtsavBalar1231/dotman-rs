#!/usr/bin/env bash

if [ ! -f /etc/arch-release ]; then
	curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain nightly

	source "${HOME}"/.cargo/env
else
	sudo pacman -S rustup --noconfirm
fi
