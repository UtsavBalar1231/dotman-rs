#!/usr/bin/env bash

# Set local timezone
export TZ="Asia/Kolkata"

# Setup build environment: {{{
echo "########## Setting up build environment ###########"
bash "$(pwd)"/scripts/setup_git.sh
bash "$(pwd)"/scripts/setup_env.sh
# }}}

# Install necessary packages: {{{
echo "########## Installing necessary packages ###########"
# Install necessary packages
sudo apt-get update -y && sudo apt-get upgrade -y
sudo apt install \
	fd-find \
	fzf \
	tmux \
	zsh \
	-y
# }}}

# Check if $DISPLAY is set
if [ -z "$DISPLAY" ]; then
	# Configure polybar: {{{
	echo "########## Configuring polybar ###########"
	sudo pacman --noconfirm -S polybar
	# }}}
fi

# Install btop
echo "########## Installing btop ###########"
ARCH=$(uname -m)
if [ ! "$(which btop)" ]; then
	if [ "${ARCH}" = "x86_64" ]; then
		sudo cp "$(pwd)"/prebuilts/btop-x86_64 /usr/local/bin/btop
	elif [ "${ARCH}" = "aarch64" ]; then
		sudo cp "$(pwd)"/prebuilts/btop-aarch64 /usr/local/bin/btop
	else
		echo "btop not available for ${ARCH}"
	fi
fi

# Install diff-so-fancy: {{{
echo "########## Installing diff-so-fancy ###########"
if [ ! "$(which diff-so-fancy)" ]; then
	wget https://github.com/so-fancy/diff-so-fancy/releases/download/v1.4.3/diff-so-fancy
	chmod +x "$(pwd)"/diff-so-fancy
	sudo mv "$(pwd)"/diff-so-fancy /usr/local/bin/
fi
# }}}

# NVIM configuration: {{{
echo "########## Configuring NVIM ###########"
curl -LO https://github.com/neovim/neovim/releases/latest/download/nvim.appimage
chmod u+x nvim.appimage
sudo mv nvim.appimage /usr/local/bin/nvim

set -u
sudo update-alternatives --install /usr/bin/ex ex "${CUSTOM_NVIM_PATH}" 110
sudo update-alternatives --install /usr/bin/vi vi "${CUSTOM_NVIM_PATH}" 110
sudo update-alternatives --install /usr/bin/view view "${CUSTOM_NVIM_PATH}" 110
sudo update-alternatives --install /usr/bin/vim vim "${CUSTOM_NVIM_PATH}" 110
sudo update-alternatives --install /usr/bin/vimdiff vimdiff "${CUSTOM_NVIM_PATH}" 110

if [ -x "$(command -v luarocks)" ]; then
	sudo luarocks install luacheck
fi

sudo ln -s ~/.config/nvim/ /root/.config/nvim

# run packersync
nvim --headless +PackerSync +qa
# }}}

# Configure zsh: {{{
echo "########## Configuring zsh ###########"
sudo chsh "$(whoami)" -s /bin/zsh
sudo chsh -s /bin/zsh

echo "DO!:"
echo -e "\033[1;32msource ${HOME}/.zshrc\033[0m"
# }}}

# Setup gitlint: {{{
echo "########## Configuring gitlint ###########"
if [ -f "${HOME}"/configs/gitlint ]; then
	mv "${HOME}"/configs/gitlint "${HOME}"/.gitlint
fi
# }}}

# Setup fonts: {{{
echo "########## Configuring fonts ###########"
if [ ! -d "${HOME}"/.local/share/fonts ]; then
	mkdir -p "${HOME}"/.local/share/fonts
fi
unzip "$(pwd)"/ubuntu/FireCode.zip -d "${HOME}"/.local/share/fonts
unzip "$(pwd)"/ubuntu/Twilio-Sans-Mono.zip -d "${HOME}"/.local/share/fonts
# }}}

zsh
