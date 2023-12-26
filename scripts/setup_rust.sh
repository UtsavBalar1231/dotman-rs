#!/usr/bin/env bash


if [ ! -f /etc/arch-release ]; then
	curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain nightly

	if [ ! -f ~/.cargo/env ]; then
		echo "Failed to install rustup"
		exit 1
	fi
	source "${HOME}"/.cargo/env
else
	sudo pacman -S rustup --noconfirm

	rustup toolchain install nightly --profile minimal
fi
