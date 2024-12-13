#!/usr/bin/env bash

CMD=$(realpath "${0}")
CUR_DIR=$(dirname "${CMD}")

# shellcheck source=scripts/utils.sh
source "${CUR_DIR}"/scripts/utils.sh

# shellcheck source=scripts/setup_shell.sh
source "${CUR_DIR}"/scripts/setup_shell.sh

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

# Install diff-so-fancy: {{{
if ! command -v diff-so-fancy >/dev/null 2>&1; then
	echo "Setting up diff-so-fancy..."
	diff_so_fancy_version=$(get_git_version "so-fancy/diff-so-fancy")

	curl -sLo ./diff-so-fancy https://github.com/so-fancy/diff-so-fancy/releases/download/"${diff_so_fancy_version}"/diff-so-fancy

	chmod a+x ./diff-so-fancy

	sudo mv ./diff-so-fancy /usr/local/bin/diff-so-fancy
fi
# }}}

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
	sudo cp -afr ~/.config/nvim/ /root/.config/nvim
fi

# run packersync
nvim --headless +PackerSync +qa
# }}}

# Configure zsh: {{{
sudo chsh "$(whoami)" -s "$(which zsh)"
sudo chsh -s "$(which zsh)"

if [ ! -d /root/.config/zsh ]; then
	sudo cp -afr ~/.config/zsh/ /root/.config/zsh
fi

# shellcheck disable=SC1090
source ~/.zshrc
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

exec $(which zsh)
