#!/usr/bin/env bash

# Set local timezone
export TZ="Asia/Kolkata"

DEBIAN_VER=$(cat /etc/debian_version)

if [ -e "/etc/debian_version" ]; then
	echo -e "Debian ${DEBIAN_VER} detected"
	if (( $(echo "${DEBIAN_VER}" -gt 10 |bc -l) )); then
		exit 1
	else
		# Setup build environment
		bash $(pwd)/scripts/setup_git.sh
		bash $(pwd)/scripts/setup_env.sh
	fi
fi

# Install necessary packages
sudo apt-get update
sudo apt-get install \
	fd-find \
	fzf \
	neovim \
	tmux \
	thefuck \
	zsh \
	-y

# Configure tmux
cp $(pwd)/.tmux.conf ~/

# Configure bat
arch=$(dpkg --print-architecture)

function get_latest_release() {
	curl --silent "https://api.github.com/repos/$1/releases/latest" | # Get latest release from GitHub api
		grep '"tag_name":' |                                          # Get tag line
		sed -E 's/.*"([^"]+)".*/\1/'                                  # Pluck JSON value
	}

function bat_install() {
	VRELEASE=$(get_latest_release 'sharkdp/bat')
	RELEASE=$(echo ${VRELEASE} | sed 's/v0/0/g')
	ARCHIVE=bat_${RELEASE}_${1}.deb
	wget https://github.com/sharkdp/bat/releases/download/${VRELEASE}/${ARCHIVE}
	sudo dpkg -i ${ARCHIVE}
	rm -f ${ARCHIVE}
}

if [ ! $(which bat) ]; then
	bat_install ${arch}
	$(which bat) --generate-config-file
	cp batconfig ~/.config/bat/config
fi

# Install gotop
if [ ! $(which gotop) ]; then
	git clone --depth=1 https://github.com/cjbassi/gotop /tmp/gotop
	/tmp/gotop/scripts/download.sh
	sudo install $(pwd)/gotop /usr/local/bin/gotop
	if [ -f $(pwd)/gotop ]; then
		rm -f $(pwd)/gotop
	fi
fi

# Install micro editor
if [ ! $(which micro) ]; then
	curl https://getmic.ro | bash
	sudo install micro /usr/local/bin/micro
	if [ -f $(pwd)/micro ]; then
		rm -f $(pwd)/micro
	fi
fi

# Install zenith
function zenith_install() {
	VRELEASE=$(get_latest_release 'bvaisvil/zenith')
	ARCHIVE=zenith_${RELEASE}-1_${1}.deb
	wget https://github.com/bvaisvil/zenith/releases/download/${VRELEASE}/${ARCHIVE}
	sudo dpkg -i ${ARCHIVE}
	rm -f ${ARCHIVE}
}
if [ ! $(which zenith) ]; then
	echo
	# zenith_install ${arch}
fi

# Install diff-so-fancy
if [ ! $(which diff-so-fancy) ]; then
	wget https://github.com/so-fancy/diff-so-fancy/releases/download/v1.4.3/diff-so-fancy
	chmod +x $(pwd)/diff-so-fancy
	sudo mv $(pwd)/diff-so-fancy /usr/local/bin/
fi

#
# Configure NeoVIM
#
# Installing vim-plug
if [ ! -f "${XDG_DATA_HOME:-$HOME/.local/share}"/nvim/site/autoload/plug.vim ]; then
	curl -fLo "${XDG_DATA_HOME:-$HOME/.local/share}"/nvim/site/autoload/plug.vim --create-dirs \
		https://raw.githubusercontent.com/junegunn/vim-plug/master/plug.vim
fi

# VIM configuration
cp -vr $(pwd)/nvim/ ~/.config/

# NVIM update and install plugins
echo -e "Run nvim comand:"
echo -e "nvim +PlugInstall +PlugUpdate +PlugClean +UpdateRemotePlugins"

# Install Oh My ZSH
if [ ! -e ${HOME}/.oh-my-zsh/.oh-my-zsh.sh ]; then
	bash -c "$(curl -fsSL https://raw.github.com/ohmyzsh/ohmyzsh/master/tools/install.sh)"
fi

sudo chsh $(which zsh)
cp -v $(pwd)/.zshrc ~/.zshrc

zsh $(pwd)/setup-zsh-dependencies.sh
