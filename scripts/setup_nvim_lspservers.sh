#!/usr/bin/env bash

arch_packages="python-black flake8 clangformat-all lua-format luacheck languagetool marksman-bin dprint-bin cpplint shellcheck jq shfmt stylelint stylua vale vscode-json-languageserver yamllint"

if [ ! -f /etc/arch-release ]; then
	echo "TODO for ubuntu later"
	exit 1
else
	if ! command -v yay >/dev/null 2>&1; then
		echo "yay is not installed"
		exit 1
	fi

	yay -Syu --noconfirm

	IFS=' ' read -ra packages <<<"$arch_packages"
	for package in "${packages[@]}"; do
		if ! pacman -Qi "$package" >/dev/null 2>&1; then
			yay -S --noconfirm "$package"
		else
			echo "$package is already installed"
		fi
	done
fi
