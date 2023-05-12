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
	btop \
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
sudo apt-get install software-properties-common -y
sudo add-apt-repository ppa:neovim-ppa/stable -y
curl -s https://apt.dustinblackman.com/KEY.gpg | sudo apt-key add -
curl -s https://apt.dustinblackman.com/dustinblackman.list >/tmp/dustinblackman.list && sudo mv /tmp/dustinblackman.list /etc/apt/sources.list.d/dustinblackman.list
sudo apt-get update -y
sudo apt-get install neovim languagetool-code-comments -y

sudo luarocks install luacheck

sudo ln -s ~/.config/nvim/ /root/.config/nvim
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
