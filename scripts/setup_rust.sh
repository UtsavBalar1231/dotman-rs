#!/usr/bin/env bash

if [ ! -f /etc/arch-release ]; then
	curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain stable

	if [ ! -f ~/.cargo/env ]; then
		echo "Failed to install rustup"
		exit 1
	fi
	. "${HOME}"/.cargo/env
else
	sudo pacman -S rustup --noconfirm

	rustup toolchain install stable --profile minimal
fi
