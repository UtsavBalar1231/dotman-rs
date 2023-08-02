#!/usr/bin/env bash

CMD=$(realpath "${0}")
CUR_DIR=$(dirname "${CMD}")

# Install exa (ls replacement): {{{
if command -v exa &>/dev/null; then
	cargo install exa
fi
# }}}

# Install bat (cat replacement): {{{
if command -v bat &>/dev/null; then
	cargo install bat
fi
# }}}

# Install fd (find replacement): {{{
if command -v fd &>/dev/null; then
	cargo install fd-find
fi
# }}}

# Install ripgrep (grep replacement): {{{
if command -v rg &>/dev/null; then
	cargo install ripgrep
fi
# }}}

get_ubuntu_version() {
	lsb_release -ds | cut -d ' ' -f 2 | cut -d '.' -f 1
}

# Install btop
ARCH=$(uname -m)
if ! command -v btop &>/dev/null; then
	if [[ $(get_ubuntu_version) -lt 23 ]]; then
		if [ "${ARCH}" = "x86_64" ]; then
			sudo cp -f "${CUR_DIR}"/prebuilts/btop-x86_64 /usr/local/bin/btop
		elif [ "${ARCH}" = "aarch64" ]; then
			sudo cp -f "${CUR_DIR}"/prebuilts/btop-aarch64 /usr/local/bin/btop
		else
			echo "btop not available for ${ARCH}"
		fi
	else
		sudo apt install -y btop
	fi
fi

# NVIM configuration: {{{
curl -sLo ./nvim https://github.com/neovim/neovim/releases/latest/download/nvim.appimage
chmod a+x ./nvim

CUSTOM_NVIM_PATH=/usr/local/bin/nvim
sudo mv ./nvim ${CUSTOM_NVIM_PATH}

set -u
sudo update-alternatives --install /usr/bin/ex ex "${CUSTOM_NVIM_PATH}" 110
sudo update-alternatives --install /usr/bin/vi vi "${CUSTOM_NVIM_PATH}" 110
sudo update-alternatives --install /usr/bin/view view "${CUSTOM_NVIM_PATH}" 110
sudo update-alternatives --install /usr/bin/vim vim "${CUSTOM_NVIM_PATH}" 110
sudo update-alternatives --install /usr/bin/vimdiff vimdiff "${CUSTOM_NVIM_PATH}" 110

if ! command -v luarocks &>/dev/null; then
	sudo luarocks install luacheck
fi

sudo ln -s ~/.config/nvim/ /root/.config/nvim

# run packersync
nvim --headless +PackerSync +qa
# }}}

# Configure zsh: {{{
sudo chsh "$(whoami)" -s /bin/zsh
sudo chsh -s /bin/zsh

echo "DO!:"
echo -e "\033[1;32msource ${HOME}/.zshrc\033[0m"
# }}}

# Setup fonts: {{{
if [ ! -d "${HOME}"/.local/share/fonts ]; then
	mkdir -p "${HOME}"/.local/share/fonts
fi
unzip "${CUR_DIR}"/ubuntu/FireCode.zip -d "${HOME}"/.local/share/fonts
unzip "${CUR_DIR}"/ubuntu/Twilio-Sans-Mono.zip -d "${HOME}"/.local/share/fonts
# }}}

# Install nodejs
if [[ $(get_ubuntu_version) -lt 22 ]]; then
	curl -fsSL https://deb.nodesource.com/setup_19.x | sudo -E bash -
	sudo apt-get install -y nodejs npm
else
	sudo apt-get install -y nodejs npm
fi

zsh
