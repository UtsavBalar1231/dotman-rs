#!/usr/bin/env bash

CMD=$(realpath "${0}")
CUR_DIR=$(dirname "${CMD}")

# Set local timezone
export TZ="Asia/Kolkata"

# Setup build environment: {{{
bash "${CUR_DIR}"/scripts/setup_git.sh
bash "${CUR_DIR}"/scripts/setup_env.sh
# }}}

# Install necessary packages: {{{
# Install necessary packages
sudo apt-get update -y && sudo apt-get upgrade -y
sudo apt install -y \
	fd-find \
	fzf \
	tmux \
	zsh
# }}}

# Install btop
ARCH=$(uname -m)
if ! command -v btop &>/dev/null; then
	if [ "${ARCH}" = "x86_64" ]; then
		sudo cp -f "${CUR_DIR}"/prebuilts/btop-x86_64 /usr/local/bin/btop
	elif [ "${ARCH}" = "aarch64" ]; then
		sudo cp -f "${CUR_DIR}"/prebuilts/btop-aarch64 /usr/local/bin/btop
	else
		echo "btop not available for ${ARCH}"
	fi
fi

get_git_version() {
	local git_package="${1}"

	curl --silent "https://api.github.com/repos/${git_package}/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/'
}

# Install diff-so-fancy: {{{
if ! command -v diff-so-fancy &>/dev/null; then
	diff_so_fancy_version=$(get_git_version "so-fancy/diff-so-fancy")

	curl -sLo /usr/local/bin/diff-so-fancy https://github.com/so-fancy/diff-so-fancy/releases/download/"${diff_so_fancy_version}"/diff-so-fancy

	chmod a+x /usr/local/bin/diff-so-fancy
fi
# }}}

# NVIM configuration: {{{
curl -sLO /usr/local/bin/nvim https://github.com/neovim/neovim/releases/latest/download/nvim.appimage
chmod a+x /usr/local/bin/nvim

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

zsh
