#!/usr/bin/env bash

set -euo pipefail

CMD=$(realpath "${0}")
CUR_DIR=$(dirname "${CMD}")

# shellcheck disable=SC1090
source "${CUR_DIR}"/utils.sh

arch_packages="python-black flake8 clangformat-all lua-format luacheck languagetool marksman-bin dprint-bin cpplint shellcheck jq shfmt stylelint stylua vale vscode-json-languageserver yamllint"

debian_packages="clang-format shellcheck jq yamllint"

install_luarocks() {
	lua_temp_dir=$(mktemp -d)
	git clone --depth=1 https://github.com/luarocks/luarocks "$lua_temp_dir"
	cd "$lua_temp_dir" || return 1
	./configure --prefix=/usr/local --sysconfdir=/etc --force-config
	make linux -j"$(nproc)"
	sudo make install -j"$(nproc)"
	cd - || return 1
	rm -rf "$lua_temp_dir"

	return 0
}

if [ "$(id -u)" -ne 0 ]; then
	echo "Please run this script as root"
	exit 1
fi

if [ ! -f /etc/arch-release ]; then
	if ! command -v python3 >/dev/null 2>&1; then
		sudo apt install -y python3
	fi

	if ! command -v npm >/dev/null 2>&1; then
		sudo apt install -y npm
	fi

	# Install black and flake8
	if ! command -v black >/dev/null 2>&1; then
		python3 -m pip install black
	fi

	if ! command -v flake8 >/dev/null 2>&1; then
		python3 -m pip install flake8
	fi

	# Install luarocks
	if command -v luarocks >/dev/null 2>&1; then
		luarocks_version="$(luarocks --version | head -n1 | cut -d ' ' -f 2 | tr -d '.')"
		echo "luarocks version: ${luarocks_version}"

		# check if luarocks version is less than 3.0.0
		if [ "${luarocks_version}" != dev ] && [ "${luarocks_version}" -lt 300 ]; then
			sudo apt remove -y luarocks
			install_luarocks
		fi
	else
		install_luarocks
	fi

	# Install lua-format and luacheck
	if ! command -v lua-format >/dev/null 2>&1; then
		sudo luarocks install --server=https://luarocks.org/dev luaformatter
	fi
	if ! command -v luacheck >/dev/null 2>&1; then
		sudo luarocks install luacheck
	fi

	if ! command -v marksman >/dev/null 2>&1; then
		marksman_version=$(get_git_version "artempyanykh/marksman")

		arch=$(uname -m)

		case $arch in
		x86_64)
			curl -sLo ./marksman https://github.com/artempyanykh/marksman/releases/download/"${marksman_version}"/marksman-linux-x64
			;;
		aarch64)
			curl -sLo ./marksman https://github.com/artempyanykh/marksman/releases/download/"${marksman_version}"/marksman-linux-arm64
			;;
			*)
			echo "Unsupported architecture"
			exit 1
			;;
		esac

		sudo mv ./marksman /usr/local/bin/marksman
		sudo chmod +x /usr/local/bin/marksman
	fi

	if ! command -v cpplint >/dev/null 2>&1; then
		pip install cpplint
	fi

	if ! command -v stylelint >/dev/null 2>&1; then
		npm install -g stylelint
	fi

	if ! command -v vale >/dev/null 2>&1; then
		vale_version=$(get_git_version "errata-ai/vale")

		arch=$(uname -m)

		case $arch in
		x86_64)
			curl -sLo ./vale.tar.gz https://github.com/errata-ai/vale/releases/download/"${vale_version}"/vale_"${vale_version}"_Linux_64-bit.tar.gz
			;;
		aarch64)
			curl -sLo ./vale.tar.gz https://github.com/errata-ai/vale/releases/download/"${vale_version}"/vale_"${vale_version}"_Linux_arm64.tar.gz
			;;
			*)
			echo "Unsupported architecture"
			exit 1
			;;
		esac

		tar -xzf ./vale.tar.gz -C /usr/local/bin
		rm -f ./vale.tar.gz
	fi

	if ! command -v prettier >/dev/null 2>&1; then
		npm install -g prettier
	fi

	if ! command -v markdownlint >/dev/null 2>&1; then
		npm install -g markdownlint-cli
	fi

	sudo apt update -y

	IFS=' ' read -ra packages <<<"$debian_packages"
	for package in "${packages[@]}"; do
		if ! dpkg -s "$package" >/dev/null 2>&1; then
			sudo apt install -y "$package"
		else
			echo "$package is already installed"
		fi
	done
else
	if ! command -v yay >/dev/null 2>&1; then
		echo "yay is not installed"
		exit 1
	fi

	yay -Sy --noconfirm

	IFS=' ' read -ra packages <<<"$arch_packages"
	for package in "${packages[@]}"; do
		if ! pacman -Qi "$package" >/dev/null 2>&1; then
			yay -S --noconfirm "$package"
		else
			echo "$package is already installed"
		fi
	done
fi
