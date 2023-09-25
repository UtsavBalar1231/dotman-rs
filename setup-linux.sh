#!/usr/bin/env bash

CMD=$(realpath "${0}")
CUR_DIR=$(dirname "${CMD}")

# shellcheck disable=SC1090
source "${CUR_DIR}"/scripts/utils.sh

if command -v eza &>/dev/null; then
	cargo install eza
fi

if command -v bat &>/dev/null; then
	cargo install bat
fi

if command -v fd &>/dev/null; then
	cargo install fd-find
fi

if command -v rg &>/dev/null; then
	cargo install ripgrep
fi

if ! command -v dprint >/dev/null 2>&1; then
	cargo install dprint
fi

if ! command -v stylua >/dev/null 2>&1; then
	cargo install stylua
fi

if ! command -v git-delta >/dev/null 2>&1; then
	cargo install git-delta
fi

if ! command -v cargo-bloat >/dev/null 2>&1; then
	cargo install cargo-bloat
fi

if ! command -v cargo-bump >/dev/null 2>&1; then
	cargo install cargo-bump
fi

if ! command -v cargo-update >/dev/null 2>&1; then
	cargo install cargo-update
fi

if ! command -v dysk >/dev/null 2>&1; then
	cargo install dysk
fi

if ! command -v fcp >/dev/null 2>&1; then
	cargo install fcp
fi

if ! command -v broot >/dev/null 2>&1; then
	cargo install broot
fi

if ! command -v svls >/dev/null 2>&1; then
	cargo install svls
fi

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

if [ ! -d /root/.config ]; then
	sudo mkdir -p /root/.config
fi

if [ ! -d /root/.config/nvim ]; then
	sudo ln -s ~/.config/nvim/ /root/.config/nvim
fi

# run packersync
nvim --headless +PackerSync +qa
# }}}

# Configure zsh: {{{
sudo chsh "$(whoami)" -s /bin/zsh
sudo chsh -s /bin/zsh

if [ ! -d /root/.config/zsh ]; then
	sudo ln -s ~/.config/zsh/ /root/.config/zsh
fi

echo "DO!:"
echo -e "\033[1;32msource ${HOME}/.zshrc\033[0m"
# }}}

# Setup fonts: {{{
if [ ! -d "${HOME}"/.local/share/fonts ]; then
	mkdir -p "${HOME}"/.local/share/fonts
fi
unzip "${CUR_DIR}"/ubuntu/FireCode.zip -d "${HOME}"/.local/share/fonts
unzip "${CUR_DIR}"/ubuntu/Twilio-Sans-Mono.zip -d "${HOME}"/.local/share/fonts

fc-cache -f -v
# }}}

# Install nodejs
if [[ $(get_ubuntu_version) -lt 22 ]]; then
	curl -fsSL https://deb.nodesource.com/setup_19.x | sudo -E bash -
	sudo apt-get install -y nodejs npm
else
	sudo apt-get install -y nodejs npm
fi

zsh
